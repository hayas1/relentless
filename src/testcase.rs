use crate::error::{HttpError, RelentlessError, RelentlessResult};
use config::{Config, Format, Setting, Testcase};
use reqwest::{Client, Request};
use std::{path::Path, str::FromStr};
use tokio::task::JoinSet;
use tower::{timeout::TimeoutLayer, Layer, Service};

pub mod config;

#[derive(Debug, Clone)]
pub struct Worker<S, L> {
    pub service: S,
    pub layer: L,
}
impl<S: Service<Request>, L: Layer<S>> Worker<S, L>
where
    L::Service: Service<Request>,
{
    pub fn new(service: S, layer: L) -> Self {
        Self { service, layer }
    }

    pub async fn run(
        self,
        req: Request,
    ) -> Result<
        <<L as Layer<S>>::Service as Service<Request>>::Response,
        <<L as Layer<S>>::Service as Service<Request>>::Error,
    > {
        let mut worker = self.layer.layer(self.service);
        worker.call(req).await
    }
}
impl<S, L> Worker<S, L>
where
    S: Service<Request> + Clone + Send + 'static,
    L: Layer<S> + Clone + Send + 'static,
    L::Service: Service<Request>,
    <L as Layer<S>>::Service: Send,
    <<L as Layer<S>>::Service as Service<Request>>::Future: Send,
    <<L as Layer<S>>::Service as Service<Request>>::Response: Send + 'static,
    <<L as Layer<S>>::Service as Service<Request>>::Error: Send + 'static,
    RelentlessError: From<<<L as Layer<S>>::Service as Service<Request>>::Error>,
{
    pub async fn run_chunk(
        self,
        chunk: Chunk<L>,
    ) -> RelentlessResult<Vec<<<L as Layer<S>>::Service as Service<Request>>::Response>> {
        let mut join_set = JoinSet::new();
        for req in chunk.req {
            let worker = self.clone();
            join_set.spawn(async move { worker.run(req).await });
        }

        let mut response = Vec::new();
        while let Some(res) = join_set.join_next().await {
            response.push(res??);
        }
        Ok(response)
    }
}

#[derive(Debug)]
pub struct Chunk<L> {
    pub description: Option<String>,
    pub req: Vec<Request>,
    pub layer: Option<L>,
}
impl<L: Layer<reqwest::Client>> Chunk<L> {
    pub fn new(description: Option<String>, req: Vec<Request>, layer: Option<L>) -> Self {
        Self {
            description,
            req,
            layer,
        }
    }
}

impl Config {
    pub fn import<P: AsRef<Path>>(path: P) -> RelentlessResult<Self> {
        Ok(Format::from_path(path.as_ref())?.import_testcase(path.as_ref())?)
    }

    pub async fn run(&self) -> RelentlessResult<()> {
        let chunks = self.testcase.iter().map(|h| self.chunk(h));

        let worker = self.worker();
        for chunk in chunks {
            let _res = worker.clone().run_chunk(chunk?).await?;
        }
        Ok(())
    }

    pub fn worker(&self) -> Worker<Client, TimeoutLayer> {
        let client = reqwest::Client::new();
        let timeout = TimeoutLayer::new(self.setting.timeout);
        Worker::new(client, timeout)
    }

    pub fn chunk(&self, testcase: &Testcase) -> RelentlessResult<Chunk<TimeoutLayer>> {
        let description = testcase.description.clone();
        let requests = Self::to_requests(&self.setting, testcase)?;

        Ok(Chunk::new(description, requests, None))
    }

    pub fn to_requests(setting: &Setting, testcase: &Testcase) -> RelentlessResult<Vec<Request>> {
        Ok(setting
            .origin
            .values()
            .map(|origin| {
                let method = reqwest::Method::from_str(&testcase.method)?;
                let url = reqwest::Url::parse(origin)?.join(&testcase.pathname)?;
                Ok::<_, HttpError>(Request::new(method, url))
            })
            .collect::<Result<Vec<_>, _>>()?)
    }
}
