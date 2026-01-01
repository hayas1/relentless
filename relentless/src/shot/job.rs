use std::fmt::{Debug, Display};
use std::str::FromStr;
use std::{fs::File, path::PathBuf};

#[cfg(feature = "cli")]
use clap::{Args, Parser};
use futures::{StreamExt, TryStreamExt};
use http::Uri;
use semigroup::{CombineIterator, Lazy, Semigroup};
use serde::{Deserialize, Serialize};
use tower::Layer;
use tower::{MakeService, Service};

use crate::report::ReportSpec;
use crate::{
    report::ReportFormat,
    shot::{
        contract::{Contract, Evaluated, RequestSource, ResponseSink, ServiceError, SignContract},
        destinations::Destinations,
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
    pub async fn job<C, Q, P>() -> crate::Result<(Job<C, Q, P>, JobSpec)>
    where
        C: for<'x> Deserialize<'x> + Default,
        Q: for<'x> Deserialize<'x> + Default,
        P: for<'x> Deserialize<'x> + Default,
    {
        let cli = Self::parse();
        let suites = Job::from_files(&cli.file)?;
        Ok((suites, cli.job))
    }
    #[cfg(feature = "cli")]
    pub async fn run<F, T, C, Q, P>(f: F) -> Result<T, Box<dyn std::error::Error>>
    where
        F: AsyncFnOnce(Job<C, Q, P>, JobSpec) -> Result<T, Box<dyn std::error::Error>>,
        C: for<'x> Deserialize<'x> + Default,
        Q: for<'x> Deserialize<'x> + Default,
        P: for<'x> Deserialize<'x> + Default,
    {
        let otel = crate::otel::Otel;
        let provider = otel.provider()?;
        otel.init_tracing(&provider)?;
        otel.set_global_propagator();
        let res = {
            let span = tracing::info_span!("run");
            let _enter = span.enter();
            let (suites, job) = Self::job::<C, Q, P>().await?;
            f(suites, job).await?
        };
        provider.force_flush()?;
        provider.shutdown()?;

        Ok(res)
    }
}

#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
#[cfg_attr(feature = "cli", derive(Args))]
pub struct JobSpec {
    /// override destinations
    #[cfg_attr(feature = "cli", arg(short, long, num_args=0.., value_parser = Cli::separated::<String, '=', String>))]
    pub destination: Vec<(String, String)>,

    /// spec of report
    #[cfg_attr(feature = "cli", command(flatten))]
    pub report_spec: ReportSpec,

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
impl JobSpec {
    pub fn destinations<U: Clone + Into<Uri>>(
        &self,
        destinations: &Destinations<U>,
    ) -> Result<Lazy<Destinations<Uri>>, <Uri as FromStr>::Err> {
        let overwrite: Result<Destinations<_>, _> =
            self.destination.iter().map(|(d, u)| u.parse().map(|u| (d, u))).collect();
        let base: Destinations<_> = destinations.iter().map(|(d, u)| (d, u.clone().into())).collect();
        Ok(Lazy::from(base).semigroup(overwrite?.into()))
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Job<C, Q, P>(pub Vec<SuiteCase<C, Q, P>>);
impl<C, Q, P> Job<C, Q, P>
where
    C: for<'x> Deserialize<'x> + Default,
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
pub struct JobReport<'a, C, Q, P, M> {
    pub suites: Vec<SuiteReport<'a, C, Q, P, M>>,
    pub evaluated: Evaluated,
}
impl<S, Q, P> Job<S, Q, P> {
    #[tracing::instrument(name = "job", skip(make_service))]
    pub async fn shot<M, T, C>(
        &self,
        make_service: M,
        job: &JobSpec,
    ) -> crate::Result<JobReport<'_, S, Q, P, P::Message>>
    where
        M: Clone + MakeService<http::Uri, C::TransportReq, Service = T>,
        T: Clone + Service<C::TransportReq, Response = C::TransportRes> + Send,
        S: Debug + SignContract<T, C> + Default,
        C: Contract<T, Sign = S, ReqSource = Q, ResSink = P> + Layer<T>,
        C::Service: Clone + Service<C::Request, Response = C::Response> + Send,
        Q: Debug + Clone + Semigroup + RequestSource<C::Request>,
        P: Debug + Clone + Semigroup + ResponseSink<Result<C::Response, ServiceError<T, C>>>,
        P::Message: Display,
    {
        let buffers = if Hierarchy::Job.contains(&job.sequential) { 1 } else { self.0.len().max(1) };
        let suites: Vec<_> = futures::stream::iter(&self.0)
            .map(|sc| sc.shot(make_service.clone(), job))
            .buffer_unordered(buffers)
            .try_collect()
            .await?;
        let evaluated = suites.iter().map(|s| s.evaluated.clone()).combine();
        Ok(JobReport { suites, evaluated })
    }
}
