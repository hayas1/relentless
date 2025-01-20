use std::{fmt::Display, io::Write, path::PathBuf, process::ExitCode};

#[cfg(feature = "cli")]
use clap::{Parser, ValueEnum};
use serde::{Deserialize, Serialize};
use tower::{Service, ServiceBuilder};

#[cfg(feature = "console-report")]
use crate::interface::report::console::ConsoleReport;
use crate::{
    assault::{
        destinations::Destinations,
        evaluate::Evaluate,
        factory::RequestFactory,
        reportable::{Report, ReportWriter, Reportable},
        service::record::{CollectClone, IoRecord, RecordLayer, RecordService, RequestIoRecord},
        worker::Control,
    },
    error::{InterfaceError, IntoResult},
};

use super::{
    config::{Config, Configuration},
    helper::{coalesce::Coalesce, http_serde_priv},
    report::github_markdown::GithubMarkdownReport,
};

#[allow(async_fn_in_trait)] // TODO #[warn(async_fn_in_trait)] by default
pub trait Assault<Req, Res> {
    type Request: Configuration + Coalesce + RequestFactory<Req>;
    type Response: Configuration + Coalesce + Evaluate<Res>;
    type Recorder;

    fn command(&self) -> &Relentless;
    fn recorder(&self) -> Self::Recorder;

    #[cfg(feature = "cli")]
    async fn execute<S>(&self, service: S) -> crate::Result<ExitCode>
    where
        <Self::Request as RequestFactory<Req>>::Error: std::error::Error + Send + Sync + 'static,
        <Self::Response as Evaluate<Res>>::Message: Display,
        S: Service<Req, Response = Res> + Clone + Send + 'static,
        S::Error: std::error::Error + Send + Sync + 'static,
        S::Future: Send + 'static,
    {
        let Relentless { rps, .. } = self.command();
        if rps.is_some() {
            unimplemented!("`--rps` is not implemented yet");
        }

        let (configs, errors) = self.configs();
        errors.into_iter().for_each(|err| eprintln!("{}", err));

        let report = self.assault_with(configs, service).await?;
        self.report_with(&report, std::io::stdout()).box_err()?;

        Ok(self.exit_code(&report))
    }

    /// TODO document
    #[allow(clippy::type_complexity)] // TODO default: #[warn(clippy::type_complexity)]
    fn configs(&self) -> (Vec<Config<Self::Request, Self::Response>>, Vec<crate::Error>) {
        let Relentless { file, .. } = self.command();
        let (ok, err): (Vec<_>, _) = file.iter().map(Config::read).partition(Result::is_ok);
        let (configs, errors) =
            (ok.into_iter().map(Result::unwrap).collect(), err.into_iter().map(Result::unwrap_err).collect());
        (configs, errors)
    }
    fn all_destinations(&self, configs: &[Config<Self::Request, Self::Response>]) -> Vec<http::Uri> {
        let d = self.command().destinations().unwrap_or_default();
        configs
            .iter()
            .flat_map(|c| c.worker_config.destinations.values())
            .chain(d.values())
            .map(|o| (**o).clone())
            .collect()
    }

    /// TODO document
    // TODO return type should be `impl Service<Req>` ?
    fn build_service<S>(&self, service: S) -> RecordService<S, Self::Recorder>
    where
        Self::Recorder: IoRecord<Req>
            + CollectClone<Req>
            + RequestIoRecord<Req>
            + IoRecord<Res>
            + CollectClone<Res>
            + Clone
            + Send
            + 'static,
        S: Service<Req>,
    {
        // TODO use option_layer ?
        ServiceBuilder::new()
            .layer(RecordLayer::new(self.command().output_record.clone(), self.recorder()))
            .service(service)
    }
    async fn assault_with<S>(
        &self,
        configs: Vec<Config<Self::Request, Self::Response>>,
        service: S,
    ) -> crate::Result<Report<<Self::Response as Evaluate<Res>>::Message, Self::Request, Self::Response>>
    where
        <Self::Request as RequestFactory<Req>>::Error: std::error::Error + Send + Sync + 'static,
        S: Service<Req, Response = Res> + Clone + Send + 'static,
        S::Error: std::error::Error + Send + Sync + 'static,
        S::Future: Send + 'static,
    {
        let cmd = self.command();
        let control = Control::new(service);
        let report = control.assault(cmd, configs).await?;
        Ok(report)
    }

    // fn report(
    //     &self,
    //     report: &Report<<Self::Response as Evaluate<Res>>::Message, Self::Request, Self::Response>,
    // ) -> Result<ExitCode, std::fmt::Error>
    // where
    //     <Self::Response as Evaluate<Res>>::Message: Display,
    // {
    //     self.report_with(report, std::io::stdout())
    // }
    fn report_with<W>(
        &self,
        report: &Report<<Self::Response as Evaluate<Res>>::Message, Self::Request, Self::Response>,
        mut write: W,
    ) -> Result<bool, std::fmt::Error>
    where
        <Self::Response as Evaluate<Res>>::Message: Display,
        W: Write,
    {
        let cmd = self.command();
        let Relentless { no_color, report_format, .. } = cmd;
        #[cfg(feature = "console-report")]
        console::set_colors_enabled(!no_color);

        match (report.skip_report(cmd), report_format) {
            (false, ReportFormat::NullDevice) => (),
            #[cfg(feature = "console-report")]
            (false, ReportFormat::Console) => report.console_report(cmd, &mut ReportWriter::new(0, &mut write))?,
            (false, ReportFormat::GithubMarkdown) => {
                report.github_markdown_report(cmd, &mut ReportWriter::new(0, &mut write))?
            }
            _ => (),
        };

        Ok(report.allow(cmd.strict))
    }

    fn pass(&self, report: &Report<<Self::Response as Evaluate<Res>>::Message, Self::Request, Self::Response>) -> bool {
        report.pass()
    }
    fn allow(
        &self,
        report: &Report<<Self::Response as Evaluate<Res>>::Message, Self::Request, Self::Response>,
    ) -> bool {
        report.allow(self.command().strict)
    }
    fn exit_code(
        &self,
        report: &Report<<Self::Response as Evaluate<Res>>::Message, Self::Request, Self::Response>,
    ) -> ExitCode {
        report.exit_code(self.command())
    }
}

#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
#[cfg_attr(feature = "cli", derive(Parser))]
#[cfg_attr(feature = "cli", clap(version, about, arg_required_else_help = true))]
pub struct Relentless {
    /// config files of testcases
    #[cfg_attr(feature = "cli", arg(short, long, num_args=0.., value_delimiter = ' '))]
    pub file: Vec<PathBuf>,

    /// override destinations
    #[cfg_attr(feature = "cli", arg(short, long, num_args=0.., value_parser = Self::parse_key_value::<String, String>, number_of_values=1))]
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
    #[cfg(feature = "cli")]
    pub fn parse_cli() -> Self {
        Self::parse()
    }
    #[cfg(feature = "cli")]
    pub fn try_parse_cli() -> crate::Result<Self> {
        Self::try_parse().box_err()
    }
    #[cfg(feature = "cli")]
    pub fn parse_key_value<T, U>(s: &str) -> crate::Result<(T, U)>
    where
        T: std::str::FromStr,
        T::Err: std::error::Error + Send + Sync + 'static,
        U: std::str::FromStr,
        U::Err: std::error::Error + Send + Sync + 'static,
    {
        let (name, destination) = s.split_once('=').ok_or_else(|| InterfaceError::KeyValueFormat(s.to_string()))?;
        Ok((name.parse().box_err()?, destination.parse().box_err()?))
    }

    pub fn destinations(&self) -> crate::Result<Destinations<http_serde_priv::Uri>> {
        let Self { destination, .. } = self;
        destination.iter().map(|(k, v)| Ok((k.to_string(), http_serde_priv::Uri(v.parse().box_err()?)))).collect()
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

    // #[test]
    // #[cfg(all(feature = "yaml", feature = "json"))]
    // fn test_read_configs_filtered() {
    //     let cmd = Relentless {
    //         file: glob::glob("tests/config/parse/*.yaml").unwrap().collect::<Result<Vec<_>, _>>().unwrap(),
    //         report_format: ReportFormat::NullDevice,
    //         ..Default::default()
    //     };
    //     let (configs, e) = cmd.configs::<HttpRequest, HttpResponse>();
    //     assert_eq!(configs.len(), glob::glob("tests/config/parse/valid_*.yaml").unwrap().filter(Result::is_ok).count());

    //     let mut buf = Vec::new();
    //     for err in e {
    //         writeln!(buf, "{}", err).unwrap();
    //     }
    //     let warn = String::from_utf8_lossy(&buf);
    //     assert!(warn.contains("tests/config/parse/invalid_simple_string.yaml"));
    //     assert!(warn.contains("tests/config/parse/invalid_different_struct.yaml"));
    //     assert!(warn.contains(
    //         &[
    //             r#"[tests/config/parse/invalid_simple_string.yaml] invalid type: string "simple string yaml", expected struct Config"#,
    //             r#""#,
    //         ]
    //         .join("\n")
    //     ));
    // }
}
