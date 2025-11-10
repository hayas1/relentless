use std::path::PathBuf;

use clap::{Args, Parser, ValueEnum};
use serde::{Deserialize, Serialize};

#[cfg(feature = "cli")]
use crate::error::CommandError;

#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
#[cfg_attr(feature = "cli", derive(Parser))]
#[cfg_attr(feature = "cli", command(version, about, arg_required_else_help = true))]

pub struct Cli {
    /// config files of testsuites
    #[cfg_attr(feature = "cli", arg(num_args=0.., value_delimiter = ' '))]
    pub file: Vec<PathBuf>,

    /// config files of jobs
    #[cfg_attr(feature = "cli", command(flatten))]
    pub job: JobSpec,
}
impl Cli {
    #[cfg(feature = "cli")]
    pub fn separated<T, const D: char, U>(s: &str) -> Result<(T, U), CommandError>
    where
        T: for<'a> From<&'a str>,
        U: for<'a> From<&'a str>,
    {
        let (k, value) =
            s.split_once(D).ok_or_else(|| CommandError::InvalidKeyValueFormat { delim: D, got: s.to_string() })?;
        Ok((k.into(), value.into()))
    }
}

#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
#[cfg_attr(feature = "cli", derive(Args))]
pub struct JobSpec {
    /// override destinations
    #[cfg_attr(feature = "cli", arg(short, long, num_args=0.., value_parser = Cli::separated::<String, '=', String>))]
    pub destination: Vec<(String, String)>,

    /// allow invalid testcases
    #[cfg_attr(feature = "cli", arg(env, long))]
    pub strict: bool,

    /// report only failed testcases
    #[cfg_attr(feature = "cli", arg(env, long))]
    pub ng_only: bool,

    /// without colorize output
    #[cfg_attr(feature = "cli", arg(env, long))]
    pub no_color: bool,

    /// format of report
    #[cfg_attr(feature = "cli", arg(env, short, long, value_enum, default_value_t))]
    pub report_format: ReportFormat,

    /// *EXPERIMENTAL* output directory
    #[cfg_attr(feature = "cli", arg(env, short, long))]
    pub output_record: bool,

    /// requests per second
    #[cfg_attr(feature = "cli", arg(env, long))]
    pub rps: Option<f64>,

    /// duration
    #[cfg_attr(feature = "cli", arg(env, long))]
    pub duration: Option<u64>, // TODO Duration
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
