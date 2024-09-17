use error::RelentlessError;
use reqwest::{Request, Response};
use tower::Service;

pub mod config;
pub mod error;
pub mod outcome;
pub mod worker;

#[derive(Debug, Clone)]
pub struct Relentless<S = reqwest::Client> {
    configs: Vec<config::Config>,
    client: Option<S>,
}
impl Relentless<reqwest::Client> {
    /// TODO document
    pub fn read_paths<I: IntoIterator<Item = P>, P: AsRef<std::path::Path>>(paths: I) -> error::RelentlessResult<Self> {
        let configs = paths.into_iter().map(config::Config::read).collect::<error::RelentlessResult<Vec<_>>>()?;
        Ok(Self::new(configs, None))
    }
    /// TODO document
    pub fn read_dir<P: AsRef<std::path::Path>>(path: P) -> error::RelentlessResult<Self> {
        let configs = config::Config::read_dir(path)?;
        Ok(Self::new(configs, None))
    }
}
impl<S> Relentless<S>
where
    S: Clone + Service<Request, Response = Response> + Send + 'static,
    S::Future: Send + 'static,
    S::Error: Send + 'static,
    RelentlessError: From<S::Error>,
{
    /// TODO document
    pub fn new(configs: Vec<config::Config>, client: Option<S>) -> Self {
        Self { configs, client }
    }
    /// TODO document
    pub async fn assault(self) -> error::RelentlessResult<Outcome> {
        let Self { configs, client } = self;
        let mut outcomes = Vec::new();
        // TODO async
        for config in configs {
            let (worker, cases) = config.instance(client.clone())?;
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
    pub fn exit_code(&self, strict: bool) -> std::process::ExitCode {
        (!self.outcome.iter().all(|o| o.allow(strict)) as u8).into()
    }
}
