use std::{collections::HashMap, path::PathBuf, process::ExitCode};

#[cfg(feature = "cli")]
use clap::{ArgGroup, Parser, Subcommand};
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
    let ret = cmd.execute().await?;
    Ok(ret.exit_code())
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
#[cfg_attr(feature = "cli", derive(Parser))]
#[cfg_attr(feature = "cli", clap(version, about, arg_required_else_help = true))]
pub struct Relentless {
    /// subcommand
    #[cfg_attr(feature = "cli", clap(subcommand))]
    pub cmd: Cmd,

    /// without colorize output
    #[cfg_attr(feature = "cli", arg(long, global = true))]
    pub no_color: bool,
}
impl Relentless {
    pub async fn execute(self) -> RelentlessResult<CmdRet> {
        let Self { cmd, no_color } = self;
        console::set_colors_enabled(!no_color);

        let ret = match cmd {
            Cmd::Assault(assault) => CmdRet::Assault(assault.execute().await?),
        };

        Ok(ret)
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
        match &self.cmd {
            Cmd::Assault(assault) => assault.execute_with(services).await,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "cli", derive(Subcommand))]
pub enum Cmd {
    /// run testcases
    #[cfg_attr(feature = "cli", clap(arg_required_else_help = true))]
    Assault(Assault),
}
impl Default for Cmd {
    fn default() -> Self {
        Self::Assault(Default::default())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CmdRet {
    Assault(Outcome),
}
impl CmdRet {
    pub fn exit_code(&self) -> ExitCode {
        match self {
            CmdRet::Assault(outcome) => outcome.exit_code(false),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
#[cfg_attr(feature = "cli", derive(Parser))]
#[cfg_attr(feature = "cli", clap(group(ArgGroup::new("files").args(&["file"]).conflicts_with("configs_dir"))))]
pub struct Assault {
    /// config files of testcases
    #[cfg_attr(feature = "cli", arg(short, long, num_args=0..))]
    pub file: Vec<PathBuf>,

    /// directory of config files
    #[cfg_attr(feature = "cli", arg(short, long))]
    pub configs_dir: Option<PathBuf>,

    /// override destinations
    #[cfg_attr(feature = "cli", arg(short, long, num_args=0.., value_parser = parse_key_value::<String, String>, number_of_values=1))]
    pub destination: Vec<(String, String)>, // TODO HashMap<String, Uri>, but clap won't parse HashMap

    /// allow invalid testcases
    #[cfg_attr(feature = "cli", arg(short, long))]
    pub strict: bool,

    /// report only failed testcases
    #[cfg_attr(feature = "cli", arg(long))]
    pub ng_only: bool,

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
impl From<Assault> for Cmd {
    fn from(assault: Assault) -> Self {
        Self::Assault(assault)
    }
}
impl Assault {
    pub fn configs(&self) -> RelentlessResult<Vec<Config>> {
        let Self { file, configs_dir, .. } = self;

        // TODO error handling
        let configs = if let Some(dir) = configs_dir {
            Config::read_dir(dir)?
        } else {
            file.iter().map(Config::read).collect::<RelentlessResult<Vec<_>>>()?
        };
        Ok(configs)
    }
    pub async fn execute(&self) -> RelentlessResult<Outcome> {
        let configs = self.configs()?;
        let outcome = self.execute_with(Control::default_http_clients(self, &configs).await?).await?;
        Ok(outcome)
    }
    pub async fn execute_with<S, ReqB, ResB>(&self, services: Vec<HashMap<String, S>>) -> RelentlessResult<Outcome>
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
        let configs = self.configs()?;
        let control = Control::with_service(configs, services)?;
        let outcome = control.assault(self).await?;
        if !self.no_report {
            outcome.report(self)?;
        }
        Ok(outcome)
    }

    pub fn override_destination(&self, other: &HashMap<String, String>) -> HashMap<String, String> {
        let mut map = other.clone();
        for (name, dest) in &self.destination {
            map.entry(name.to_string()).and_modify(|d| *d = dest.to_string());
        }
        map
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
    fn test_exclude_file_or_dir() {
        let Err(_) = Relentless::try_parse_from(["relentless", "assault"]) else {
            panic!("file or directory must be specified");
        };

        match Relentless::try_parse_from(["relentless", "assault", "--file", "examples/config/assault.yaml"]) {
            Ok(cli) => assert_eq!(
                cli.cmd,
                Cmd::Assault(Assault {
                    file: vec![PathBuf::from("examples/config/assault.yaml")],
                    configs_dir: None,
                    ..Default::default()
                })
            ),
            Err(_) => panic!("only file is allowed"),
        };
        match Relentless::try_parse_from([
            "relentless",
            "assault",
            "--file",
            "examples/config/assault.yaml",
            "--file",
            "examples/config/compare.yaml",
        ]) {
            Ok(cli) => assert_eq!(
                cli.cmd,
                Cmd::Assault(Assault {
                    file: vec![
                        PathBuf::from("examples/config/assault.yaml"),
                        PathBuf::from("examples/config/compare.yaml")
                    ],
                    configs_dir: None,
                    ..Default::default()
                })
            ),
            Err(_) => panic!("multiple file is allowed"),
        };

        match Relentless::try_parse_from(["relentless", "assault", "--configs-dir", "examples/config"]) {
            Ok(cli) => assert_eq!(
                cli.cmd,
                Cmd::Assault(Assault {
                    file: Vec::new(),
                    configs_dir: Some(PathBuf::from("examples/config")),
                    ..Default::default()
                })
            ),
            Err(_) => panic!("only configs_dir is allowed"),
        };

        let Err(_) = Relentless::try_parse_from([
            "relentless",
            "assault",
            "--file",
            "examples/config/assault.yaml",
            "--configs-dir",
            "examples/config",
        ]) else {
            panic!("dir and file are exclusive");
        };
    }

    #[test]
    #[cfg(feature = "cli")]
    fn test_no_color_arg_position() {
        match Relentless::try_parse_from(["relentless", "assault", "-c", "examples/config"]) {
            Ok(cli) => assert!(!cli.no_color),
            Err(_) => panic!("--no-color is optional, default is false"),
        }
        match Relentless::try_parse_from(["relentless", "--no-color", "assault", "-c", "examples/config"]) {
            Ok(cli) => assert!(cli.no_color),
            Err(_) => panic!("--no-color is main command option"),
        };
        match Relentless::try_parse_from(["relentless", "assault", "-c", "examples/config", "--no-color"]) {
            Ok(cli) => assert!(cli.no_color),
            Err(_) => panic!("--no-color is main command option, but it is global"),
        };
    }
}
