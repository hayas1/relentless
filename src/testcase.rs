use crate::error::RelentlessResult;
use config::{Config, Format};
use reqwest::{Client, Request};
use std::path::Path;
use tower::{timeout::TimeoutLayer, Layer, Service};

pub mod config;
pub mod http;

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
        let mut client = self.layer.layer(self.service);
        client.call(req).await
    }
}

impl Config {
    pub fn import<P: AsRef<Path>>(path: P) -> RelentlessResult<Self> {
        Ok(Format::from_path(path.as_ref())?.import_testcase(path.as_ref())?)
    }

    pub async fn run(&self) -> RelentlessResult<()> {
        let requests = self
            .testcase
            .iter()
            .flat_map(|h| self.setting.origin.values().map(|host| h.to_request(host))); // TODO do not flatten (for compare test)

        let worker = self.worker();
        for r in requests {
            let _res = worker.clone().run(r?).await?;
        }
        Ok(())
    }

    pub fn worker(&self) -> Worker<Client, TimeoutLayer> {
        let client = reqwest::Client::new();
        let timeout = TimeoutLayer::new(self.setting.timeout);
        Worker::new(client, timeout)
    }
}
