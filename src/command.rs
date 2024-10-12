use std::{path::PathBuf, process::ExitCode};

#[cfg(feature = "cli")]
use clap::Parser;
use http_body::Body;
use serde::{Deserialize, Serialize};
use tower::Service;

use crate::{
    config::{Config, Destinations},
    error::{RelentlessError, RelentlessResult, RelentlessResult_, RunCommandError, RunCommandResult},
    outcome::{Evaluator, Outcome},
    service::FromBodyStructure,
    worker::Control,
};

#[cfg(feature = "cli")]
pub async fn execute() -> Result<ExitCode, Box<dyn std::error::Error + Send + Sync>> {
    let cmd = Relentless::parse();

    let &Relentless { number_of_threads, rps, .. } = &cmd;
    if number_of_threads.is_some() {
        unimplemented!("`--number-of-threads` is not implemented yet");
    }
    if rps.is_some() {
        unimplemented!("`--rps` is not implemented yet");
    }

    let ret = cmd.assault().await?;
    Ok(ret.exit_code(cmd))
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

    /// report nothing
    #[cfg_attr(feature = "cli", arg(long))]
    pub no_report: bool,

    /// number of threads
    #[cfg_attr(feature = "cli", arg(short, long))]
    pub number_of_threads: Option<usize>,

    /// requests per second
    #[cfg_attr(feature = "cli", arg(short, long))]
    pub rps: Option<usize>,
}
impl Relentless {
    /// TODO document
    pub fn configs(&self) -> RunCommandResult<Vec<Config>> {
        let Self { file, .. } = self;
        let (ok, err): (_, Vec<_>) = file.iter().map(Config::read).partition(Result::is_ok);
        let (config, errors): (_, Vec<_>) = (
            ok.into_iter().map(Result::unwrap).collect(),
            err.into_iter().map(Result::unwrap_err).map(Box::new).collect(),
        );
        if errors.is_empty() {
            Ok(config)
        } else {
            Err(RunCommandError::CannotReadSomeConfigs(config, errors))
        }
    }

    /// TODO document
    #[cfg(feature = "default-http-client")]
    pub async fn assault(&self) -> RelentlessResult_<Outcome> {
        let configs = match self.configs() {
            Ok(configs) => configs,
            Err(RunCommandError::CannotReadSomeConfigs(configs, _)) if !self.strict => configs, // TODO strict ? warning ?
            Err(e) => return Err(e)?,
        };
        let clients = Control::default_http_clients(self, &configs).await?;
        let outcome = self.assault_with::<_, _, _, crate::outcome::DefaultEvaluator>(configs, clients).await?;
        Ok(outcome)
    }
    /// TODO document
    pub async fn assault_with<S, ReqB, ResB, E>(
        &self,
        configs: Vec<Config>,
        services: Vec<Destinations<S>>,
    ) -> RelentlessResult<Outcome>
    where
        ReqB: Body + FromBodyStructure + Send + 'static,
        ReqB::Data: Send + 'static,
        ReqB::Error: std::error::Error + Sync + Send + 'static,
        ResB: Body + Send + 'static,
        ResB::Data: Send + 'static,
        ResB::Error: std::error::Error + Sync + Send + 'static,
        S: Service<http::Request<ReqB>, Response = http::Response<ResB>> + Send + Sync + 'static,
        E: Evaluator<http::Response<ResB>>,
        RelentlessError: From<S::Error> + From<E::Error>,
    {
        let Self { no_color, no_report, .. } = self;
        console::set_colors_enabled(!no_color);

        let control = Control::<_, _, _, E>::with_service(self, configs, services)?;
        let outcome = control.assault().await?;
        if !no_report {
            outcome.report(self)?;
        }
        Ok(outcome)
    }
}

#[cfg(feature = "cli")]
pub fn parse_key_value<T, U>(s: &str) -> Result<(T, U), RunCommandError>
where
    T: std::str::FromStr,
    T::Err: std::error::Error + Send + Sync + 'static,
    RunCommandError: From<T::Err>,
    U: std::str::FromStr,
    U::Err: std::error::Error + Send + Sync + 'static,
    RunCommandError: From<U::Err>,
{
    let (name, destination) = s.split_once('=').ok_or_else(|| RunCommandError::KeyValueFormat(s.to_string()))?;
    Ok((name.parse()?, destination.parse()?))
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

        match Relentless::try_parse_from(["relentless", "--file", "examples/config/*.yaml", "--file"]) {
            Ok(cmd) => assert_eq!(
                cmd,
                Relentless {
                    file: vec![
                        // WARN: * may be wildcard in shell, clap doesn't support it
                        PathBuf::from("examples/config/*.yaml"),
                        // PathBuf::from("examples/config/assault.yaml"),
                        // PathBuf::from("examples/config/compare.yaml")
                    ],
                    ..Default::default()
                }
            ),
            Err(_) => panic!("specify multiple files should be ok"),
        };
    }
}
