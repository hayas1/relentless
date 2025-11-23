use std::{fs::File, path::PathBuf};

#[cfg(feature = "cli")]
use clap::{Args, Parser};
use futures::{StreamExt, TryStreamExt};
use serde::{Deserialize, Serialize};
use tower::{MakeService, Service};

#[cfg(feature = "cli")]
use crate::shot::contract::{Contract, RequestSource};
use crate::{
    report::ReportFormat,
    shot::{
        contract::ResponseSink,
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
    pub job: JobSpec,
}
impl Cli {
    #[cfg(feature = "cli")]
    pub fn separated<T, const D: char, U>(s: &str) -> Result<(T, U), crate::error::CommandError>
    where
        T: for<'x> From<&'x str>,
        U: for<'x> From<&'x str>,
    {
        let (key, value) = s
            .split_once(D)
            .ok_or_else(|| crate::error::CommandError::InvalidKeyValueFormat { delim: D, got: s.to_string() })?;
        Ok((key.into(), value.into()))
    }
    #[cfg(feature = "cli")]
    pub async fn shot<M, S, C>(make_service: M) -> crate::Result<JobReport<C::ReqSource, C::ResSink>>
    where
        M: Clone + MakeService<http::Uri, C::TransportReq, Service = S>,
        S: Clone + Service<C::TransportReq, Response = C::TransportRes> + Send,
        C: Contract<S>,
        C::Service: Service<C::Request, Response = C::Response, Error = C::ServiceError> + Send,
        C::ReqSource: for<'x> Deserialize<'x> + Default + RequestSource<C::Request> + 'static,
        C::ResSink: for<'x> Deserialize<'x> + Default + ResponseSink<Result<C::Response, C::ServiceError>> + 'static,
    {
        let cli = Self::parse();
        let suites = Job::from_files(&cli.file)?;
        suites.shot::<_, _, C>(make_service, &cli.job).await
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
pub struct Job<Q, P>(pub Vec<SuiteCase<Q, P>>);
impl<Q, P> Job<Q, P>
where
    Q: for<'x> Deserialize<'x> + Default,
    P: for<'x> Deserialize<'x> + Default,
{
    pub fn from_files(files: &[PathBuf]) -> crate::Result<Self> {
        let suites: Result<Vec<_>, _> = files
            .iter()
            .map(|f| {
                let file = File::open(f).map_err(crate::Error::boxed)?;
                serde_yaml::from_reader(file).map_err(crate::Error::boxed)
            })
            .collect();
        Ok(Self(suites?))
    }
}
#[derive(Debug, Clone, PartialEq)]
pub struct JobReport<Q, P> {
    suites: Vec<SuiteReport<Q, P>>,
}
impl<Q, P> Job<Q, P> {
    pub async fn shot<M, S, C>(self, make_service: M, job: &JobSpec) -> crate::Result<JobReport<Q, P>>
    where
        M: Clone + MakeService<http::Uri, C::TransportReq, Service = S>,
        S: Clone + Service<C::TransportReq, Response = C::TransportRes> + Send,
        C: Contract<S, ReqSource = Q, ResSink = P>,
        C::Service: Service<C::Request, Response = C::Response, Error = C::ServiceError> + Send,
        Q: RequestSource<C::Request> + 'static,
        P: ResponseSink<Result<C::Response, C::ServiceError>> + 'static,
    {
        let buffers = if Hierarchy::Job.contains(&job.sequential) { 1 } else { self.0.len().max(1) };
        let suites = futures::stream::iter(self.0)
            .map(|sc| sc.shot::<_, _, C>(make_service.clone(), job))
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
