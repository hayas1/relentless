use std::{fmt::Display, io::Write, path::PathBuf, process::ExitCode};

#[cfg(feature = "cli")]
use clap::{Parser, ValueEnum};
use serde::{Deserialize, Serialize};
use tower::{Service, ServiceBuilder};

#[cfg(feature = "default-http-client")]
use crate::implement::service_http::client::DefaultHttpClient;
#[cfg(feature = "console-report")]
use crate::interface::report::console::ConsoleReport;
use crate::{
    assault::{
        destinations::Destinations,
        evaluate::Evaluate,
        factory::RequestFactory,
        reportable::{Report, ReportWriter, Reportable},
        service::record::{RecordLayer, RecordService},
        worker::Control,
    },
    error2::{InterfaceError, IntoResult},
    implement::service_http::{evaluate::HttpResponse, factory::HttpRequest},
};

use super::{config::Config, helper::http_serde_priv, report::github_markdown::GithubMarkdownReport};

#[cfg(feature = "cli")]
pub async fn execute() -> Result<ExitCode, Box<dyn std::error::Error + Send + Sync>> {
    let cmd = Relentless::parse();

    let Relentless { rps, .. } = &cmd;
    if rps.is_some() {
        unimplemented!("`--rps` is not implemented yet");
    }

    let rep = cmd.assault().await?;
    cmd.report(&rep)?;
    Ok(cmd.exit_code(&rep))
}

#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
#[cfg_attr(feature = "cli", derive(Parser))]
#[cfg_attr(feature = "cli", clap(version, about, arg_required_else_help = true))]
pub struct Relentless {
    /// config files of testcases
    #[cfg_attr(feature = "cli", arg(short, long, num_args=0.., value_delimiter = ' '))]
    pub file: Vec<PathBuf>,

    /// override destinations
    #[cfg_attr(feature = "cli", arg(short, long, num_args=0.., value_parser = parse_key_value::<String, String>, number_of_values=1))]
    pub destination: Vec<(String, String)>, // TODO HashMap<String, Uri>, but clap won't parse HashMap

    /// allow invalid testcases
    #[cfg_attr(feature = "cli", arg(long))]
    pub strict: bool,

    /// report only failed testcases
    #[cfg_attr(feature = "cli", arg(long))]
    pub ng_only: bool,

    /// without colorize output
    #[cfg_attr(feature = "cli", arg(long))]
    pub no_color: bool,

    /// format of report
    #[cfg_attr(feature = "cli", arg(short, long), clap(value_enum, default_value_t))]
    pub report_format: ReportFormat,

    /// *EXPERIMENTAL* output directory
    #[cfg_attr(feature = "cli", arg(short, long))]
    pub output_record: Option<PathBuf>,

    /// without async for each requests
    #[cfg_attr(feature = "cli", arg(short, long, num_args=0.., value_delimiter = ' '))]
    pub sequential: Vec<WorkerKind>, // TODO dedup in advance

    /// measure and report metrics for each requests
    #[cfg_attr(feature = "cli", arg(short, long, num_args=0.., value_delimiter = ' '))]
    pub measure: Option<Vec<WorkerKind>>, // TODO dedup in advance

    /// measure percentile for latency
    #[cfg_attr(feature = "cli", arg(short, long, num_args=0.., value_delimiter = ' '))]
    pub percentile: Option<Vec<f64>>, // TODO dedup in advance

    /// requests per second
    #[cfg_attr(feature = "cli", arg(long))]
    pub rps: Option<usize>,
}
#[cfg_attr(feature = "cli", derive(ValueEnum))]
#[derive(Debug, Clone, PartialEq, Eq, Default, Hash, Serialize, Deserialize)]
pub enum ReportFormat {
    /// without report
    #[cfg_attr(not(feature = "console-report"), default)]
    NullDevice,

    /// report to console
    #[cfg(feature = "console-report")]
    #[cfg_attr(feature = "console-report", default)]
    Console,

    /// report to markdown
    GithubMarkdown,
}
#[cfg_attr(feature = "cli", derive(ValueEnum))]
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Default, Hash, Serialize, Deserialize)]
pub enum WorkerKind {
    /// each repeats
    Repeats,

    /// each testcases
    Testcases,

    /// each configs
    #[default]
    Configs,
}

impl Relentless {
    pub fn destinations(&self) -> crate::Result2<Destinations<http_serde_priv::Uri>> {
        let Self { destination, .. } = self;
        destination
            .iter()
            .map(|(k, v)| Ok((k.to_string(), http_serde_priv::Uri(v.parse().box_err()?))))
            .collect::<Result<Destinations<_>, _>>()
    }

    pub fn sequential_set(&self) -> Vec<WorkerKind> {
        let mut v = self.sequential.clone();
        v.sort_unstable();
        v.dedup();
        v
    }
    pub fn is_sequential(&self, kind: WorkerKind) -> bool {
        self.sequential_set().contains(&kind)
    }

    pub fn measure_set(&self) -> Vec<WorkerKind> {
        let default = vec![WorkerKind::Configs];
        let mut v = self.measure.as_ref().unwrap_or(&default).clone();
        v.sort_unstable();
        v.dedup();
        v
    }
    pub fn is_measure(&self, apply_to: WorkerKind) -> bool {
        self.measure_set().contains(&apply_to)
    }

    pub fn percentile_set(&self) -> Vec<f64> {
        let default = vec![50., 90., 99.];
        let mut v = self.percentile.as_ref().unwrap_or(&default).clone();
        v.sort_by(|a, b| a.partial_cmp(b).unwrap_or_else(|| unreachable!("{}", InterfaceError::NanPercentile)));
        v.dedup();
        v
    }
    pub fn quantile_set(&self) -> Vec<f64> {
        self.percentile_set().iter().map(|p| p / 100.).collect()
    }

    /// TODO document
    pub fn configs(&self) -> (Vec<Config<HttpRequest, HttpResponse>>, Vec<crate::Error2>) {
        let Self { file, .. } = self;
        let (ok, err): (Vec<_>, _) = file.iter().map(Config::read).partition(Result::is_ok);
        let (configs, errors) =
            (ok.into_iter().map(Result::unwrap).collect(), err.into_iter().map(Result::unwrap_err).collect());
        (configs, errors)
    }

    /// TODO document
    // TODO return type should be `impl Service<Req>` ?
    pub fn build_service<S, Req>(&self, service: S) -> RecordService<S>
    where
        S: Service<Req>,
    {
        // TODO use option_layer ?
        ServiceBuilder::new().layer(RecordLayer::new(self.output_record.clone())).service(service)
    }

    /// TODO document
    #[cfg(all(feature = "default-http-client", feature = "cli"))]
    pub async fn assault(
        &self,
    ) -> crate::Result2<Report<crate::implement::service_http::error::HttpEvaluateError, HttpRequest, HttpResponse>>
    {
        let (configs, cannot_read) = self.configs();
        for err in cannot_read {
            eprintln!("{}", err);
        }
        let service = self.build_service(DefaultHttpClient::<reqwest::Body, reqwest::Body>::new().await?);
        let report = self.assault_with(configs, service).await?;
        Ok(report)
    }
    /// TODO document
    pub async fn assault_with<S, Req>(
        &self,
        configs: Vec<Config<HttpRequest, HttpResponse>>,
        service: S,
    ) -> crate::Result2<Report<<HttpResponse as Evaluate<S::Response>>::Message, HttpRequest, HttpResponse>>
    where
        HttpRequest: RequestFactory<Req>,
        <HttpRequest as RequestFactory<Req>>::Error: std::error::Error + Send + Sync + 'static,
        HttpResponse: Evaluate<S::Response>,
        S: Service<Req> + Clone + Send + 'static,
        S::Error: std::error::Error + Send + Sync + 'static,
        S::Future: Send + 'static,
    {
        let control = Control::new(service);
        let report = control.assault(self, configs).await?;
        Ok(report)
    }

    pub fn report<M: Display>(
        &self,
        report: &Report<M, HttpRequest, HttpResponse>,
    ) -> Result<ExitCode, std::fmt::Error> {
        self.report_with(report, std::io::stdout())
    }
    pub fn report_with<M: Display, W: Write>(
        &self,
        report: &Report<M, HttpRequest, HttpResponse>,
        mut write: W,
    ) -> Result<ExitCode, std::fmt::Error> {
        let Self { no_color, report_format, .. } = self;
        #[cfg(feature = "console-report")]
        console::set_colors_enabled(!no_color);

        match report_format {
            ReportFormat::NullDevice => (),
            #[cfg(feature = "console-report")]
            ReportFormat::Console => report.console_report(self, &mut ReportWriter::new(0, &mut write))?,
            ReportFormat::GithubMarkdown => {
                report.github_markdown_report(self, &mut ReportWriter::new(0, &mut write))?
            }
        };

        Ok(report.exit_code(self))
    }

    pub fn pass<T>(&self, report: &Report<T, HttpRequest, HttpResponse>) -> bool {
        report.pass()
    }
    pub fn allow<T>(&self, report: &Report<T, HttpRequest, HttpResponse>) -> bool {
        report.allow(self.strict)
    }
    pub fn exit_code<T>(self, report: &Report<T, HttpRequest, HttpResponse>) -> ExitCode {
        report.exit_code(&self)
    }
}

#[cfg(feature = "cli")]
pub fn parse_key_value<T, U>(s: &str) -> crate::Result2<(T, U)>
where
    T: std::str::FromStr,
    T::Err: std::error::Error + Send + Sync + 'static,
    U: std::str::FromStr,
    U::Err: std::error::Error + Send + Sync + 'static,
{
    let (name, destination) = s.split_once('=').ok_or_else(|| InterfaceError::KeyValueFormat(s.to_string()))?;
    Ok((name.parse().box_err()?, destination.parse().box_err()?))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[cfg(feature = "cli")]
    fn test_file_must_be_specified() {
        let Err(_) = Relentless::try_parse_from(["relentless"]) else {
            panic!("files must be specified");
        };

        match Relentless::try_parse_from(["relentless", "--file", "examples/config/assault.yaml"]) {
            Ok(cmd) => assert_eq!(
                cmd,
                Relentless { file: vec![PathBuf::from("examples/config/assault.yaml")], ..Default::default() }
            ),
            Err(_) => panic!("specify one file should be ok"),
        };

        match Relentless::try_parse_from([
            "relentless",
            "--file",
            "examples/config/assault.yaml",
            "--file",
            "examples/config/compare.yaml",
        ]) {
            Ok(cmd) => assert_eq!(
                cmd,
                Relentless {
                    file: vec![
                        PathBuf::from("examples/config/assault.yaml"),
                        PathBuf::from("examples/config/compare.yaml")
                    ],
                    ..Default::default()
                }
            ),
            Err(_) => panic!("specify multiple files should be ok"),
        };

        // `--file examples/config/*.yaml` will expand as this by shell
        match Relentless::try_parse_from([
            "relentless",
            "--file",
            "examples/config/assault.yaml",
            "examples/config/compare.yaml",
        ]) {
            Ok(cmd) => assert_eq!(
                cmd,
                Relentless {
                    file: vec![
                        PathBuf::from("examples/config/assault.yaml"),
                        PathBuf::from("examples/config/compare.yaml")
                    ],
                    ..Default::default()
                }
            ),
            Err(_) => panic!("specify multiple files should be ok"),
        };
    }

    #[test]
    #[cfg(feature = "cli")]
    fn test_parse_key_value_err() {
        let err_msg = Relentless::try_parse_from([
            "relentless",
            "--file",
            "examples/config/*.yaml",
            "--destination",
            "key-value",
        ])
        .unwrap_err()
        .to_string();
        assert!(err_msg.contains(&InterfaceError::KeyValueFormat("key-value".to_string()).to_string()));
    }

    #[test]
    #[cfg(feature = "cli")]
    fn test_parse_measure() {
        match Relentless::try_parse_from([
            "relentless",
            "--file",
            "examples/config/assault.yaml",
            "examples/config/compare.yaml",
        ]) {
            Ok(cmd) => assert_eq!(cmd.measure_set(), vec![WorkerKind::Configs]),
            Err(_) => panic!("no specified measure should be default value"),
        };

        match Relentless::try_parse_from([
            "relentless",
            "--file",
            "examples/config/assault.yaml",
            "examples/config/compare.yaml",
            "-m",
        ]) {
            Ok(cmd) => assert_eq!(cmd.measure_set(), vec![]),
            Err(_) => panic!("no specified measure with empty should be no measure"),
        };

        match Relentless::try_parse_from([
            "relentless",
            "--file",
            "examples/config/assault.yaml",
            "examples/config/compare.yaml",
            "--measure",
            "repeats",
            "testcases",
        ]) {
            Ok(cmd) => {
                assert_eq!(cmd.measure_set(), vec![WorkerKind::Repeats, WorkerKind::Testcases])
            }
            Err(_) => panic!("specified measure should be measured"),
        };
    }

    #[test]
    #[cfg(feature = "cli")]
    fn test_parse_percentile() {
        match Relentless::try_parse_from([
            "relentless",
            "--file",
            "examples/config/assault.yaml",
            "examples/config/compare.yaml",
        ]) {
            Ok(cmd) => assert_eq!(cmd.percentile_set(), vec![50., 90., 99.]),
            Err(_) => panic!("no specified percentile should be default value"),
        };

        match Relentless::try_parse_from([
            "relentless",
            "--file",
            "examples/config/assault.yaml",
            "examples/config/compare.yaml",
            "-p",
        ]) {
            Ok(cmd) => assert_eq!(cmd.percentile_set(), Vec::<f64>::new()),
            Err(_) => panic!("no specified percentile with empty should not be measured"),
        };

        match Relentless::try_parse_from([
            "relentless",
            "--file",
            "examples/config/assault.yaml",
            "examples/config/compare.yaml",
            "--percentile",
            "95",
            "99",
            "99.9",
        ]) {
            Ok(cmd) => assert_eq!(cmd.percentile_set(), vec![95., 99., 99.9]),
            Err(_) => panic!("specified percentile should be measured"),
        };

        match Relentless::try_parse_from([
            "relentless",
            "--file",
            "examples/config/assault.yaml",
            "examples/config/compare.yaml",
            "-p90",
            "-p99",
            "-p99.9",
        ]) {
            Ok(cmd) => assert_eq!(cmd.percentile_set(), vec![90., 99., 99.9]),
            Err(_) => panic!("specified percentile should be measured"),
        }
    }

    #[test]
    #[cfg(all(feature = "yaml", feature = "json"))]
    fn test_read_configs_filtered() {
        let cmd = Relentless {
            file: glob::glob("tests/config/parse/*.yaml").unwrap().collect::<Result<Vec<_>, _>>().unwrap(),
            report_format: ReportFormat::NullDevice,
            ..Default::default()
        };
        let (configs, e) = cmd.configs();
        assert_eq!(configs.len(), glob::glob("tests/config/parse/valid_*.yaml").unwrap().filter(Result::is_ok).count());

        let mut buf = Vec::new();
        for err in e {
            writeln!(buf, "{}", err).unwrap();
        }
        let warn = String::from_utf8_lossy(&buf);
        assert!(warn.contains("tests/config/parse/invalid_simple_string.yaml"));
        assert!(warn.contains("tests/config/parse/invalid_different_struct.yaml"));
        assert!(warn.contains(
            &[
                r#"[tests/config/parse/invalid_simple_string.yaml] invalid type: string "simple string yaml", expected struct Config"#,
                r#""#,
            ]
            .join("\n")
        ));
    }
}
