use std::{fmt::Display, io::Write, path::PathBuf, process::ExitCode};

#[cfg(feature = "cli")]
use clap::{Parser, ValueEnum};
use serde::{Deserialize, Serialize};
use tower::{Service, ServiceBuilder};

#[cfg(feature = "console-report")]
use crate::report::console_report::{ConsoleReport, ReportWriter};
use crate::{
    config::{http_serde_priv, Config, Destinations},
    error::{IntoContext, MultiWrap, RunCommandError, Wrap, WrappedResult},
    evaluate::Evaluator,
    record::{RecordLayer, RecordService},
    report::{Report, Reportable},
    service::FromRequestInfo,
    worker::Control,
};

#[cfg(feature = "cli")]
pub async fn execute() -> Result<ExitCode, Box<dyn std::error::Error + Send + Sync>> {
    let cmd = Relentless::parse();

    let Relentless { number_of_threads, rps, .. } = &cmd;
    if number_of_threads.is_some() {
        unimplemented!("`--number-of-threads` is not implemented yet");
    }
    if rps.is_some() {
        unimplemented!("`--rps` is not implemented yet");
    }

    let rep = cmd.assault().await?;
    cmd.report(&rep)?;
    Ok(cmd.exit_code(&rep))
}

#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
#[cfg_attr(feature = "cli", derive(Parser))]
#[cfg_attr(feature = "cli", clap(version, about, arg_required_else_help = true))]
pub struct Relentless {
    /// config files of testcases
    #[cfg_attr(feature = "cli", arg(short, long, num_args=0..))]
    pub file: Vec<PathBuf>,

    /// override destinations
    #[cfg_attr(feature = "cli", arg(short, long, num_args=0.., value_parser = parse_key_value::<String, String>, number_of_values=1))]
    pub destination: Vec<(String, String)>, // TODO HashMap<String, Uri>, but clap won't parse HashMap

    /// allow invalid testcases
    #[cfg_attr(feature = "cli", arg(short, long))]
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

    /// output directory
    #[cfg_attr(feature = "cli", arg(short, long))]
    pub output_record: Option<PathBuf>,

    /// number of threads
    #[cfg_attr(feature = "cli", arg(short, long))]
    pub number_of_threads: Option<usize>,

    /// requests per second
    #[cfg_attr(feature = "cli", arg(long))]
    pub rps: Option<usize>,
}
#[cfg_attr(feature = "cli", derive(ValueEnum))]
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum ReportFormat {
    /// without report
    #[cfg_attr(not(feature = "console-report"), default)]
    NullDevice,

    /// report to console
    #[cfg(feature = "console-report")]
    #[cfg_attr(feature = "console-report", default)]
    Console,
}

impl Relentless {
    pub fn destinations(&self) -> WrappedResult<Destinations<http_serde_priv::Uri>> {
        let Self { destination, .. } = self;
        destination
            .iter()
            .map(|(k, v)| Ok((k.to_string(), http_serde_priv::Uri(v.parse()?))))
            .collect::<Result<Destinations<_>, _>>()
    }

    /// TODO document
    pub fn configs(&self) -> WrappedResult<Vec<Config>> {
        let Self { file, .. } = self;
        let (ok, err): (_, Vec<_>) = file.iter().map(Config::read).partition(Result::is_ok);
        let (configs, errors): (_, MultiWrap) =
            (ok.into_iter().map(Result::unwrap).collect(), err.into_iter().map(Result::unwrap_err).collect());
        if errors.is_empty() {
            Ok(configs)
        } else {
            Err(errors.context(RunCommandError::CannotReadSomeConfigs(configs)))?
        }
    }

    /// TODO document
    pub fn configs_filtered<W: Write>(&self, mut write: W) -> WrappedResult<Vec<Config>> {
        match self.configs() {
            Ok(configs) => Ok(configs),
            Err(e) => {
                if let Some((RunCommandError::CannotReadSomeConfigs(configs), source)) =
                    e.downcast_context_ref::<_, MultiWrap>()
                {
                    writeln!(write, "{}", source)?;
                    Ok(configs.to_vec())
                } else {
                    Err(e)?
                }
            }
        }
    }

    // TODO S::Response and RecordService<S>::Response have to be the same type
    pub fn build_service<S, Req>(
        &self,
        service: S,
    ) -> Box<
        dyn Service<
                Req,
                Response = <RecordService<S> as Service<Req>>::Response,
                Error = <RecordService<S> as Service<Req>>::Error,
                Future = <RecordService<S> as Service<Req>>::Future,
            > + Send,
    >
    where
        S: Service<Req> + Send + 'static,
        RecordService<S>: Service<Req>,
    {
        Box::new(ServiceBuilder::new().layer(RecordLayer::new(self.output_record.clone())).service(service))
    }

    /// TODO document
    #[cfg(all(feature = "default-http-client", feature = "cli"))]
    pub async fn assault(&self) -> crate::Result<Report<crate::error::EvaluateError>> {
        let configs = self.configs_filtered(std::io::stderr())?;
        let mut service = self.build_service(Control::default_http_client().await?);
        let report = self.assault_with(configs, &mut service, &crate::evaluate::DefaultEvaluator).await?;
        Ok(report)
    }
    /// TODO document
    pub async fn assault_with<S, Req, E>(
        &self,
        configs: Vec<Config>,
        service: &mut S,
        evaluator: &E,
    ) -> crate::Result<Report<E::Message>>
    where
        Req: FromRequestInfo,
        S: Service<Req> + Send + 'static,
        E: Evaluator<S::Response>,
        Wrap: From<Req::Error> + From<S::Error>,
    {
        let control = Control::with_service(self, configs, service)?;
        let report = control.assault(evaluator).await?;
        Ok(report)
    }

    pub fn report<M: Display>(&self, report: &Report<M>) -> crate::Result<ExitCode> {
        self.report_with(report, std::io::stdout())
    }
    pub fn report_with<M: Display, W: Write>(&self, report: &Report<M>, mut write: W) -> crate::Result<ExitCode> {
        let Self { no_color, report_format, .. } = self;
        #[cfg(feature = "console-report")]
        console::set_colors_enabled(!no_color);

        match report_format {
            ReportFormat::NullDevice => {}
            #[cfg(feature = "console-report")]
            ReportFormat::Console => report.console_report(self, &mut ReportWriter::new(0, &mut write))?,
        };

        Ok(report.exit_code(self))
    }

    pub fn pass<T>(&self, report: &Report<T>) -> bool {
        report.pass()
    }
    pub fn allow<T>(&self, report: &Report<T>) -> bool {
        report.allow(self.strict)
    }
    pub fn exit_code<T>(self, report: &Report<T>) -> ExitCode {
        report.exit_code(&self)
    }
}

#[cfg(feature = "cli")]
pub fn parse_key_value<T, U>(s: &str) -> crate::Result<(T, U)>
where
    T: std::str::FromStr,
    T::Err: std::error::Error + Send + Sync + 'static,
    U: std::str::FromStr,
    U::Err: std::error::Error + Send + Sync + 'static,
{
    use crate::error::WithContext;

    let (name, destination) = s.split_once('=').context(RunCommandError::KeyValueFormat(s.to_string()))?; // TODO!!!
    Ok((name.parse().map_err(Wrap::wrapping)?, destination.parse().map_err(Wrap::wrapping)?))
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
        assert!(err_msg.contains(&RunCommandError::KeyValueFormat("key-value".to_string()).to_string()));
    }

    #[test]
    #[cfg(all(feature = "yaml", feature = "json"))]
    fn test_read_configs_filtered() {
        let cmd = Relentless {
            file: glob::glob("tests/config/parse/*.yaml").unwrap().collect::<Result<Vec<_>, _>>().unwrap(),
            report_format: ReportFormat::NullDevice,
            ..Default::default()
        };
        let mut buf = Vec::new();
        let configs = cmd.configs_filtered(&mut buf).unwrap();
        assert_eq!(configs.len(), glob::glob("tests/config/parse/valid_*.yaml").unwrap().filter(Result::is_ok).count());

        let warn = String::from_utf8_lossy(&buf);
        assert!(warn.contains("tests/config/parse/invalid_simple_string.yaml"));
        assert!(warn.contains("tests/config/parse/invalid_different_struct.yaml"));
        // TODO better test for error message
        assert!(warn.contains(
            &[
                r#"tests/config/parse/invalid_simple_string.yaml:"#,
                r#"invalid type: string "simple string yaml", expected struct Config"#,
                r#""#,
            ]
            .join("\n")
        ));
    }
}
