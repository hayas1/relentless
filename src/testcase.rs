use crate::error::{HttpError, RelentlessError, RelentlessResult};
use config::{Config, Format, Setting, Testcase};
use reqwest::{Client, Request, Response};
use std::{path::Path, str::FromStr};
use tokio::task::JoinSet;
use tower::{timeout::TimeoutLayer, Layer, Service};

pub mod config;

#[derive(Debug)]
pub struct Chunk<L> {
    pub description: Option<String>,
    pub req: Vec<Request>,
    pub layer: Option<L>,
    pub setting: Option<Setting>, // TODO
}
impl<L> Chunk<L> {
    pub fn new(
        description: Option<String>,
        req: Vec<Request>,
        layer: Option<L>,
        setting: Option<Setting>,
    ) -> Self {
        Self {
            description,
            req,
            layer,
            setting,
        }
    }

    pub async fn run<LW>(self, layer: Option<LW>) -> RelentlessResult<Vec<Response>>
    where
        L: Layer<Client> + Clone + Send + 'static,
        L::Service: Service<Request>,
        <L as Layer<Client>>::Service: Send,
        <<L as Layer<Client>>::Service as Service<Request>>::Future: Send,
        <<L as Layer<Client>>::Service as Service<Request>>::Response:
            Into<Response> + Send + 'static,
        <<L as Layer<Client>>::Service as Service<Request>>::Error: Send + 'static,
        LW: Layer<Client> + Clone + Send + 'static,
        LW::Service: Service<Request>,
        <LW as Layer<Client>>::Service: Send,
        <<LW as Layer<Client>>::Service as Service<Request>>::Future: Send,
        <<LW as Layer<Client>>::Service as Service<Request>>::Response:
            Into<Response> + Send + 'static,
        <<LW as Layer<Client>>::Service as Service<Request>>::Error: Send + 'static,
        RelentlessError: From<<<L as Layer<Client>>::Service as Service<Request>>::Error>
            + From<<<LW as Layer<Client>>::Service as Service<Request>>::Error>,
    {
        let mut join_set = JoinSet::<RelentlessResult<Response>>::new();
        for req in self.req {
            let mut client = Client::new();
            let (chunk_layer, worker_layer) = (self.layer.clone(), layer.clone());
            join_set.spawn(async move {
                let res = match chunk_layer {
                    Some(layer) => layer.layer(client).call(req).await?.into(),
                    None => match worker_layer {
                        Some(layer) => layer.layer(client).call(req).await?.into(),
                        None => client.call(req).await?,
                    },
                };
                Ok(res)
            });
        }

        let mut response = Vec::new();
        while let Some(res) = join_set.join_next().await {
            response.push(res??);
        }
        Ok(response)
    }
}

pub struct Worker<L> {
    pub name: Option<String>,
    pub layer: Option<L>,
    pub setting: Option<Setting>, // TODO
}
impl<L> Worker<L> {
    pub fn new(name: Option<String>, layer: Option<L>, setting: Option<Setting>) -> Self {
        Self {
            name,
            layer,
            setting,
        }
    }

    pub async fn run<LC>(self, chunks: Vec<Chunk<LC>>) -> RelentlessResult<()>
    where
        L: Layer<Client> + Clone + Send + 'static,
        L::Service: Service<Request>,
        <L as Layer<Client>>::Service: Send,
        <<L as Layer<Client>>::Service as Service<Request>>::Future: Send,
        <<L as Layer<Client>>::Service as Service<Request>>::Response:
            Into<Response> + Send + 'static,
        <<L as Layer<Client>>::Service as Service<Request>>::Error: Send + 'static,
        LC: Layer<Client> + Clone + Send + 'static,
        LC::Service: Service<Request>,
        <LC as Layer<Client>>::Service: Send,
        <<LC as Layer<Client>>::Service as Service<Request>>::Future: Send,
        <<LC as Layer<Client>>::Service as Service<Request>>::Response:
            Into<Response> + Send + 'static,
        <<LC as Layer<Client>>::Service as Service<Request>>::Error: Send + 'static,
        RelentlessError: From<<<L as Layer<Client>>::Service as Service<Request>>::Error>
            + From<<<LC as Layer<Client>>::Service as Service<Request>>::Error>,
    {
        for chunk in chunks {
            let _res = chunk.run(self.layer.clone()).await?;
        }
        Ok(())
    }
}

impl Config {
    pub fn import<P: AsRef<Path>>(path: P) -> RelentlessResult<Self> {
        Ok(Format::from_path(path.as_ref())?.import_testcase(path.as_ref())?)
    }

    pub async fn run(&self) -> RelentlessResult<()> {
        let worker = self.worker()?;
        let chunks = self
            .testcase
            .iter()
            .map(|h| self.chunk(h))
            .collect::<Result<Vec<_>, _>>();

        worker.run(chunks?).await?;
        Ok(())
    }

    pub fn worker(&self) -> RelentlessResult<Worker<TimeoutLayer>> {
        let timeout = self.setting.clone().unwrap().timeout;
        Ok(Worker::new(
            self.name.clone(),
            Some(TimeoutLayer::new(timeout)),
            self.setting.clone(),
        ))
    }

    pub fn chunk(&self, testcase: &Testcase) -> RelentlessResult<Chunk<TimeoutLayer>> {
        let description = testcase.description.clone();
        let requests = Self::to_requests(&self.setting.clone().unwrap(), testcase)?;

        Ok(Chunk::new(
            description,
            requests,
            None,
            testcase.setting.clone(),
        ))
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
