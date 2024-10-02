use std::{collections::HashMap, path::PathBuf, process::ExitCode};

#[cfg(feature = "cli")]
use clap::Parser;
use http_body::Body;
use tower::Service;

use crate::{
    config::Config,
    error::{RelentlessError, RelentlessResult},
    outcome::Outcome,
    service::FromBodyStructure,
    worker::Control,
};

#[cfg(feature = "cli")]
pub async fn execute() -> Result<ExitCode, Box<dyn std::error::Error + Send + Sync>> {
    let cmd = Relentless::parse();
    let ret = cmd.assault().await?;
    Ok(ret.exit_code(cmd))
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
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
    #[cfg_attr(feature = "cli", arg(long, global = true))]
    pub no_color: bool,

    /// report nothing
    #[cfg_attr(feature = "cli", arg(long))]
    pub no_report: bool,

    /// do not save outcomes
    #[cfg_attr(feature = "cli", arg(long))]
    pub no_save: bool,

    /// number of threads
    #[cfg_attr(feature = "cli", arg(short, long))]
    pub number_of_threads: Option<usize>,
}
impl Relentless {
    pub fn configs(&self) -> RelentlessResult<Vec<Config>> {
        let Self { file, .. } = self;
        file.iter().map(Config::read).collect::<RelentlessResult<Vec<_>>>()
    }
    pub async fn assault(&self) -> RelentlessResult<Outcome> {
        let configs = self.configs()?;
        let clients = Control::default_http_clients(self, &configs).await?;
        let outcome = self.assault_with(clients).await?;
        Ok(outcome)
    }
    pub async fn assault_with<S, ReqB, ResB>(&self, services: Vec<HashMap<String, S>>) -> RelentlessResult<Outcome>
    where
        ReqB: Body + FromBodyStructure + Send + 'static,
        ReqB::Data: Send + 'static,
        ReqB::Error: std::error::Error + Sync + Send + 'static,
        ResB: Body + Send + 'static,
        ResB::Data: Send + 'static,
        ResB::Error: std::error::Error + Sync + Send + 'static,
        S: Service<http::Request<ReqB>, Response = http::Response<ResB>> + Send + Sync + 'static,
        RelentlessError: From<S::Error>,
    {
        let Self { no_color, no_report, .. } = self;
        console::set_colors_enabled(!no_color);

        let configs = self.configs()?;
        let control = Control::with_service(self, configs, services)?;
        let outcome = control.assault(self).await?;
        if !no_report {
            outcome.report(self)?;
        }
        Ok(outcome)
    }
}

#[cfg(feature = "cli")]
pub fn parse_key_value<T, U>(s: &str) -> Result<(T, U), Box<dyn std::error::Error + Send + Sync + 'static>>
where
    T: std::str::FromStr,
    T::Err: std::error::Error + Send + Sync + 'static,
    U: std::str::FromStr,
    U::Err: std::error::Error + Send + Sync + 'static,
{
    let (name, destination) = s.split_once('=').ok_or_else(|| format!("invalid KEY=value: no `=` found in `{}`", s))?;
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
