use std::{path::PathBuf, process::ExitCode};

#[cfg(feature = "cli")]
use clap::{ArgGroup, Parser, Subcommand};

use crate::Relentless;

#[cfg(feature = "cli")]
pub async fn execute() -> Result<ExitCode, Box<dyn std::error::Error + Send + Sync>> {
    use crate::outcome::OutcomeFormatter;

    let cli = Cli::parse();
    match cli.subcommand {
        SubCommands::Assault(assault) => {
            let Assault { configs, dir_config, .. } = &assault;
            let relentless = if let Some(dir) = dir_config {
                Relentless::read_dir(dir).await?
            } else {
                Relentless::read_paths(configs).await?
            };
            let outcome = relentless.assault().await?;

            let mut show = OutcomeFormatter::new(0);
            outcome.show(&mut show, &assault)?;
            println!("{}", show);
            Ok(outcome.exit_code(assault.strict))
        }
    }
}

#[derive(Debug, PartialEq, Eq)]
#[cfg_attr(feature = "cli", derive(Parser))]
#[cfg_attr(feature = "cli", clap(version, about, arg_required_else_help = true))]
pub struct Cli {
    #[cfg_attr(feature = "cli", clap(subcommand))]
    pub subcommand: SubCommands,
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
#[cfg_attr(feature = "cli", clap(group(ArgGroup::new("config").args(&["configs"]).conflicts_with("dir_config"))))]
pub struct Assault {
    /// config files of testcases
    #[cfg_attr(feature = "cli", arg(short, long, num_args=0..))]
    pub configs: Vec<PathBuf>,

    /// directory of config files
    #[cfg_attr(feature = "cli", arg(short, long))]
    pub dir_config: Option<PathBuf>,

    /// allow invalid testcases
    #[cfg_attr(feature = "cli", arg(short, long, default_value_t = false))]
    pub strict: bool,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[cfg(feature = "cli")]
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