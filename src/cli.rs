use std::{path::PathBuf, process::ExitCode};

use clap::{ArgGroup, Parser, Subcommand};

use crate::Relentless;

pub async fn run() -> Result<ExitCode, Box<dyn std::error::Error + Send + Sync>> {
    let cli = Cli::parse();
    match cli.subcommand {
        SubCommands::Assault(Assault { configs, dir_config, strict }) => {
            let relentless = if let Some(dir) = dir_config {
                Relentless::read_dir(dir).await?
            } else {
                Relentless::read_paths(configs).await?
            };
            let outcome = relentless.assault().await?;
            Ok(outcome.exit_code(strict))
        }
    }
}

#[derive(Parser, Debug, PartialEq, Eq)]
#[clap(version, about, arg_required_else_help = true)]
pub struct Cli {
    #[clap(subcommand)]
    subcommand: SubCommands,
}

#[derive(Debug, Subcommand, PartialEq, Eq)]
pub enum SubCommands {
    /// run testcases
    #[clap(arg_required_else_help = true)]
    Assault(Assault),
}

#[derive(Parser, Debug, PartialEq, Eq, Default)]
#[clap(group(ArgGroup::new("config").args(&["configs"]).conflicts_with("dir_config")))]
pub struct Assault {
    /// config files of testcases
    #[arg(short, long, num_args=0..)]
    configs: Vec<PathBuf>,

    /// directory of config files
    #[arg(short, long)]
    dir_config: Option<PathBuf>,

    /// allow invalid testcases
    #[arg(short, long, default_value_t = false)]
    strict: bool,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn exclude_configs_or_dir() {
        let Err(_) = Cli::try_parse_from(["relentless", "assault"]) else {
            panic!("dir config or configs must be specified");
        };

        match Cli::try_parse_from(["relentless", "assault", "--configs", "examples/config/assault.yaml"]) {
            Ok(cli) => assert_eq!(
                cli.subcommand,
                SubCommands::Assault(Assault {
                    configs: vec![PathBuf::from("examples/config/assault.yaml")],
                    dir_config: None,
                    ..Default::default()
                })
            ),
            Err(_) => panic!("only configs is allowed"),
        };
        match Cli::try_parse_from([
            "relentless",
            "assault",
            "--configs",
            "examples/config/assault.yaml",
            "--configs",
            "examples/config/compare.yaml",
        ]) {
            Ok(cli) => assert_eq!(
                cli.subcommand,
                SubCommands::Assault(Assault {
                    configs: vec![
                        PathBuf::from("examples/config/assault.yaml"),
                        PathBuf::from("examples/config/compare.yaml")
                    ],
                    dir_config: None,
                    ..Default::default()
                })
            ),
            Err(_) => panic!("multiple configs is allowed"),
        };

        match Cli::try_parse_from(["relentless", "assault", "--dir-config", "examples/config"]) {
            Ok(cli) => assert_eq!(
                cli.subcommand,
                SubCommands::Assault(Assault {
                    configs: Vec::new(),
                    dir_config: Some(PathBuf::from("examples/config")),
                    ..Default::default()
                })
            ),
            Err(_) => panic!("only dir_config is allowed"),
        };

        let Err(_) = Cli::try_parse_from([
            "relentless",
            "assault",
            "--configs",
            "examples/config/assault.yaml",
            "--dir-config",
            "examples/config",
        ]) else {
            panic!("dir config and configs are exclusive");
        };
    }
}
