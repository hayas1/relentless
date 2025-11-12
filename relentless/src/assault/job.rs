use std::path::PathBuf;

use clap::{Args, Parser};
use serde::{Deserialize, Serialize};

use crate::report::ReportFormat;

#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
#[cfg_attr(feature = "cli", derive(Parser))]
#[cfg_attr(feature = "cli", command(version, about, arg_required_else_help = true))]

pub struct Cli {
    /// config files of testsuites
    #[cfg_attr(feature = "cli", arg(num_args=0.., value_delimiter = ' '))]
    pub file: Vec<PathBuf>,

    /// Spec of a job
    #[cfg_attr(feature = "cli", command(flatten))]
    pub job: JobSpec,
}
impl Cli {
    #[cfg(feature = "cli")]
    pub fn separated<T, const D: char, U>(s: &str) -> Result<(T, U), crate::error::CommandError>
    where
        T: for<'a> From<&'a str>,
        U: for<'a> From<&'a str>,
    {
        let (key, value) = s
            .split_once(D)
            .ok_or_else(|| crate::error::CommandError::InvalidKeyValueFormat { delim: D, got: s.to_string() })?;
        Ok((key.into(), value.into()))
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
