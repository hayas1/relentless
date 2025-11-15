use std::{fs::File, path::PathBuf};

#[cfg(feature = "cli")]
use clap::{Args, Parser};
use futures::{StreamExt, TryStreamExt};
use serde::{Deserialize, Serialize};

use crate::{
    report::ReportFormat,
    shot::{
        hierarchy::Hierarchy,
        suite::{SuiteCase, SuiteReport},
    },
};

#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
#[cfg_attr(feature = "cli", derive(Parser))]
#[cfg_attr(feature = "cli", command(version, about, arg_required_else_help = true))]

pub struct Cli {
    /// config files of testsuites
    #[cfg_attr(feature = "cli", arg(num_args=0.., value_delimiter = ' '))]
    pub file: Vec<PathBuf>,

    /// spec of a job
    #[cfg_attr(feature = "cli", command(flatten))]
    pub job: Job,
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
    #[cfg(feature = "cli")]
    pub async fn shot<S, Q, P>(service: S) -> crate::Result<JobReport<Q, P>>
    where
        S: Clone + Send + 'static,
        Q: for<'a> Deserialize<'a> + Default + Send + Sync + 'static,
        P: for<'a> Deserialize<'a> + Default + Send + Sync + 'static,
    {
        let cli = Self::parse();
        let suites = SuiteCases::from_files(&cli.file)?;
        suites.shot(service, &cli.job).await
    }
}

#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
#[cfg_attr(feature = "cli", derive(Args))]
pub struct Job {
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

    /// record output
    #[cfg_attr(feature = "cli", arg(env, short, long))]
    pub output_record: bool,

    /// without async for each requests
    #[cfg_attr(feature = "cli", arg(env, short, long, num_args=0.., value_delimiter = ' '))]
    pub sequential: Vec<Hierarchy>, // TODO dedup in advance

    /// requests per second
    #[cfg_attr(feature = "cli", arg(env, long))]
    pub rps: Option<f64>,

    /// duration
    #[cfg_attr(feature = "cli", arg(env, long))]
    pub duration: Option<u64>, // TODO Duration
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SuiteCases<Q, P>(pub Vec<SuiteCase<Q, P>>);
impl<Q, P> SuiteCases<Q, P>
where
    Q: for<'a> Deserialize<'a> + Default,
    P: for<'a> Deserialize<'a> + Default,
{
    pub fn from_files(files: &[PathBuf]) -> crate::Result<Self> {
        let suites: Result<Vec<_>, _> = files
            .iter()
            .map(|f| serde_yaml::from_reader(File::open(f).map_err(crate::Error::boxed)?).map_err(crate::Error::boxed))
            .collect();
        Ok(Self(suites?))
    }
}
#[derive(Debug, Clone, PartialEq)]
pub struct JobReport<Q, P> {
    suites: Vec<SuiteReport<Q, P>>,
}
impl<Q, P> SuiteCases<Q, P> {
    pub async fn shot<S>(self, service: S, job: &Job) -> crate::Result<JobReport<Q, P>>
    where
        S: Clone + Send + 'static,
        Q: Send + Sync + 'static,
        P: Send + Sync + 'static,
    {
        let buffers = if Hierarchy::Job.contains(&job.sequential) { 1 } else { self.0.len().max(1) };
        let suites = futures::stream::iter(self.0)
            .map(|sc| sc.shot(service.clone(), job))
            .buffer_unordered(buffers)
            .try_collect()
            .await?;
        Ok(JobReport { suites })
    }
}
impl<Q, P> JobReport<Q, P> {
    pub fn pass(&self) -> bool {
        true
    }
}
