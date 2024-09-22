use std::collections::HashMap;

use bytes::Bytes;
use config::{BodyStructure, FromBodyStructure};
use error::RelentlessError;
use http_body_util::{combinators::UnsyncBoxBody, Empty};
use hyper::body::{Body, Incoming};
use service::HyperClient;
use tower::Service;
use worker::Worker;

pub mod config;
pub mod error;
pub mod outcome;
pub mod service;
pub mod worker;

pub type Relentless = Relentless_<
    HyperClient<UnsyncBoxBody<Bytes, RelentlessError>, Bytes>,
    UnsyncBoxBody<Bytes, RelentlessError>,
    Bytes,
>;

#[derive(Debug, Clone)]
pub struct Relentless_<S = HyperClient<Bytes, Bytes>, ReqB = Bytes, ResB = Bytes> {
    configs: Vec<config::Config>,
    workers: Vec<Worker<S, ReqB, ResB>>, // TODO all worker do not have same clients type ?
    phantom: std::marker::PhantomData<(ReqB, ResB)>,
}
impl<ReqB> Relentless_<HyperClient<ReqB, Bytes>, ReqB, Bytes>
where
    ReqB: Body + FromBodyStructure + Send + 'static,
    ReqB::Data: Send + 'static,
    ReqB::Error: std::error::Error + Sync + Send + 'static,
{
    /// TODO document
    pub async fn with_hyper_client(configs: Vec<config::Config>) -> error::RelentlessResult<Self> {
        let mut workers = Vec::new();
        for config in configs.clone() {
            workers.push(Worker::with_hyper_client(config.worker_config).await?);
        }
        Ok(Self::new(configs, workers))
    }
    /// TODO document
    pub async fn read_paths<I: IntoIterator<Item = P>, P: AsRef<std::path::Path>>(
        paths: I,
    ) -> error::RelentlessResult<Self> {
        let configs = paths.into_iter().map(config::Config::read).collect::<error::RelentlessResult<Vec<_>>>()?;
        Self::with_hyper_client(configs).await
    }
    /// TODO document
    pub async fn read_dir<P: AsRef<std::path::Path>>(path: P) -> error::RelentlessResult<Self> {
        let configs = config::Config::read_dir(path)?;
        Self::with_hyper_client(configs).await
    }
}
impl<S, ReqB, ResB> Relentless_<S, ReqB, ResB>
where
    ReqB: Body + FromBodyStructure + Send + 'static,
    ReqB::Data: Send + 'static,
    ReqB::Error: std::error::Error + Sync + Send + 'static,
    ResB: From<Bytes> + Send + 'static,
    S: Service<http::Request<ReqB>, Response = http::Response<ResB>> + Send + Sync + 'static,
    RelentlessError: From<S::Error>,
{
    /// TODO document
    pub fn new(configs: Vec<config::Config>, workers: Vec<Worker<S, ReqB, ResB>>) -> Self {
        let phantom = std::marker::PhantomData;
        Self { configs, workers, phantom }
    }
    /// TODO document
    pub async fn assault(self) -> error::RelentlessResult<Outcome> {
        let Self { configs, workers, .. } = self;
        let mut outcomes = Vec::new();
        // TODO async
        for (config, mut worker) in configs.into_iter().zip(workers.into_iter()) {
            let cases = config.testcase.clone().into_iter().map(worker::Case::new).collect::<Vec<_>>();
            outcomes.push(worker.assault(cases).await?);
        }
        Ok(Outcome::new(outcomes))
    }
}

#[derive(Debug, Clone)]
pub struct Outcome {
    outcome: Vec<outcome::WorkerOutcome>,
}
impl Outcome {
    pub fn new(outcome: Vec<outcome::WorkerOutcome>) -> Self {
        Self { outcome }
    }
    pub fn pass(&self) -> bool {
        self.outcome.iter().all(|o| o.pass())
    }
    #[cfg(feature = "cli")]
    pub fn exit_code(&self, strict: bool) -> std::process::ExitCode {
        (!self.outcome.iter().all(|o| o.allow(strict)) as u8).into()
    }
}
