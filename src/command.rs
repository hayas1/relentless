use std::{collections::HashMap, path::PathBuf, process::ExitCode};

#[cfg(feature = "cli")]
use clap::{ArgGroup, Parser, Subcommand};

use crate::{context::ContextBuilder, error::RelentlessResult, Relentless};

#[cfg(feature = "cli")]
pub async fn execute() -> Result<ExitCode, Box<dyn std::error::Error + Send + Sync>> {
    let cmd = Cmd::parse();
    let status = cmd.run().await?;
    Ok(status)
}

#[derive(Debug, PartialEq, Eq)]
#[cfg_attr(feature = "cli", derive(Parser))]
#[cfg_attr(feature = "cli", clap(version, about, arg_required_else_help = true))]
pub struct Cmd {
    #[cfg_attr(feature = "cli", clap(subcommand))]
    pub subcommand: SubCommands,

    /// without colorize output
    #[cfg_attr(feature = "cli", arg(long, global = true))]
    pub no_color: bool,
}
impl Cmd {
    pub async fn run(self) -> RelentlessResult<ExitCode> {
        let ctx = ContextBuilder::from_cmd(self);
        let status = ctx.relentless().await?; // TODO subcommand

        Ok(status)
    }

    #[cfg(feature = "cli")]
    pub fn parse_key_value<T, U>(s: &str) -> Result<(T, U), Box<dyn std::error::Error + Send + Sync + 'static>>
    where
        T: std::str::FromStr,
        T::Err: std::error::Error + Send + Sync + 'static,
        U: std::str::FromStr,
        U::Err: std::error::Error + Send + Sync + 'static,
    {
        let (name, destination) =
            s.split_once('=').ok_or_else(|| format!("invalid KEY=value: no `=` found in `{}`", s))?;
        Ok((name.parse()?, destination.parse()?))
    }

    // TODO return Result
    pub fn assault(&self) -> Option<&Assault> {
        match &self.subcommand {
            SubCommands::Assault(assault) => Some(assault),
        }
    }
}

#[derive(Debug, PartialEq, Eq)]
#[cfg_attr(feature = "cli", derive(Subcommand))]
pub enum SubCommands {
    /// run testcases
    #[cfg_attr(feature = "cli", clap(arg_required_else_help = true))]
    Assault(Assault),
}

#[derive(Debug, PartialEq, Eq, Default)]
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
    #[cfg_attr(feature = "cli", arg(short, long, num_args=0.., value_parser = Cmd::parse_key_value::<String, String>, number_of_values=1))]
    pub destination: Vec<(String, String)>, // TODO HashMap<String, Uri>

    /// allow invalid testcases
    #[cfg_attr(feature = "cli", arg(short, long))]
    pub strict: bool,
}
impl Assault {
    pub fn override_destination(&self, other: &HashMap<String, String>) -> HashMap<String, String> {
        let mut map = other.clone();
        for (name, dest) in &self.destination {
            map.entry(name.to_string()).and_modify(|d| *d = dest.to_string());
        }
        map
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[cfg(feature = "cli")]
    fn test_exclude_file_or_dir() {
        let Err(_) = Cmd::try_parse_from(["relentless", "assault"]) else {
            panic!("file or directory must be specified");
        };

        match Cmd::try_parse_from(["relentless", "assault", "--file", "examples/config/assault.yaml"]) {
            Ok(cli) => assert_eq!(
                cli.subcommand,
                SubCommands::Assault(Assault {
                    file: vec![PathBuf::from("examples/config/assault.yaml")],
                    configs_dir: None,
                    ..Default::default()
                })
            ),
            Err(_) => panic!("only file is allowed"),
        };
        match Cmd::try_parse_from([
            "relentless",
            "assault",
            "--file",
            "examples/config/assault.yaml",
            "--file",
            "examples/config/compare.yaml",
        ]) {
            Ok(cli) => assert_eq!(
                cli.subcommand,
                SubCommands::Assault(Assault {
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

        match Cmd::try_parse_from(["relentless", "assault", "--configs-dir", "examples/config"]) {
            Ok(cli) => assert_eq!(
                cli.subcommand,
                SubCommands::Assault(Assault {
                    file: Vec::new(),
                    configs_dir: Some(PathBuf::from("examples/config")),
                    ..Default::default()
                })
            ),
            Err(_) => panic!("only configs_dir is allowed"),
        };

        let Err(_) = Cmd::try_parse_from([
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
        match Cmd::try_parse_from(["relentless", "assault", "-c", "examples/config"]) {
            Ok(cli) => assert!(!cli.no_color),
            Err(_) => panic!("--no-color is optional, default is false"),
        }
        match Cmd::try_parse_from(["relentless", "--no-color", "assault", "-c", "examples/config"]) {
            Ok(cli) => assert!(cli.no_color),
            Err(_) => panic!("--no-color is main command option"),
        };
        match Cmd::try_parse_from(["relentless", "assault", "-c", "examples/config", "--no-color"]) {
            Ok(cli) => assert!(cli.no_color),
            Err(_) => panic!("--no-color is main command option, but it is global"),
        };
    }
}
