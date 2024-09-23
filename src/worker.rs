use std::collections::HashMap;

use crate::{
    command::Assault,
    config::{Config, Protocol, Setting, Testcase, WorkerConfig},
    error::{HttpError, RelentlessError, RelentlessResult},
    outcome::{CaseOutcome, Compare, Evaluator, Outcome, OutcomeWriter, Status, WorkerOutcome},
    service::{BytesBody, DefaultHttpClient, FromBodyStructure},
};
use hyper::body::Body;
use tower::Service;

/// TODO document
#[derive(Debug, Clone)]
pub struct Control<S = DefaultHttpClient<BytesBody, BytesBody>, ReqB = BytesBody, ResB = BytesBody> {
    configs: Vec<Config>,                // TODO remove this ?
    workers: Vec<Worker<S, ReqB, ResB>>, // TODO all worker do not have same clients type ?
    cases: Vec<Vec<Case<S, ReqB, ResB>>>,
    phantom: std::marker::PhantomData<(ReqB, ResB)>,
}
impl<S, ReqB, ResB> Control<S, ReqB, ResB> {
    /// TODO document
    pub fn configs(&self) -> &Vec<Config> {
        &self.configs
    }
}
impl<ReqB> Control<DefaultHttpClient<ReqB, BytesBody>, ReqB, BytesBody>
where
    ReqB: Body + FromBodyStructure + Send + 'static,
    ReqB::Data: Send + 'static,
    ReqB::Error: std::error::Error + Sync + Send + 'static,
{
    /// TODO document
    pub async fn with_default_http_client(cmd: &Assault, configs: Vec<Config>) -> RelentlessResult<Self> {
        let mut workers = Vec::new();
        for config in &configs {
            workers.push(Worker::with_default_http_client(cmd, config.worker_config.clone()).await?);
        }
        Ok(Self::new(configs, workers))
    }
    /// TODO document
    pub async fn read_paths<I: IntoIterator<Item = P>, P: AsRef<std::path::Path>>(
        cmd: &Assault,
        paths: I,
    ) -> RelentlessResult<Self> {
        let configs = paths.into_iter().map(Config::read).collect::<RelentlessResult<Vec<_>>>()?;
        Self::with_default_http_client(cmd, configs).await
    }
    /// TODO document
    pub async fn read_dir<P: AsRef<std::path::Path>>(cmd: &Assault, path: P) -> RelentlessResult<Self> {
        let configs = Config::read_dir(path)?;
        Self::with_default_http_client(cmd, configs).await
    }
}
impl<S, ReqB, ResB> Control<S, ReqB, ResB>
where
    ReqB: Body + FromBodyStructure + Send + 'static,
    ReqB::Data: Send + 'static,
    ReqB::Error: std::error::Error + Sync + Send + 'static,
    ResB: Body + Send + 'static,
    ResB::Data: Send + 'static,
    ResB::Error: std::error::Error + Sync + Send + 'static,
    S: Service<http::Request<ReqB>, Response = http::Response<ResB>> + Send + Sync + 'static,
    RelentlessError: From<S::Error>,
{
    /// TODO document
    pub fn new(configs: Vec<Config>, workers: Vec<Worker<S, ReqB, ResB>>) -> Self {
        let cases = configs.iter().map(|c| c.testcase.clone().into_iter().map(Case::new).collect()).collect();
        let phantom = std::marker::PhantomData;
        Self { configs, workers, cases, phantom }
    }
    /// TODO document
    pub async fn assault(self, cmd: &Assault) -> RelentlessResult<Outcome> {
        let Self { workers, cases, .. } = self;

        let mut works = Vec::new();
        for (worker, cases) in workers.into_iter().zip(cases.into_iter()) {
            works.push(worker.assault(cases, cmd));
        }

        let mut outcomes = Vec::new();
        for work in works {
            outcomes.push(work.await?);
        }
        Ok(Outcome::new(outcomes))
    }
}

/// TODO document
#[derive(Debug, Clone)]
pub struct Worker<S, ReqB, ResB> {
    config: WorkerConfig,
    clients: HashMap<String, S>,
    phantom: std::marker::PhantomData<(ReqB, ResB)>,
}
impl<S, ReqB, ResB> Worker<S, ReqB, ResB> {
    pub fn config(&self) -> &WorkerConfig {
        &self.config
    }
}
impl<ReqB> Worker<DefaultHttpClient<ReqB, BytesBody>, ReqB, BytesBody>
where
    ReqB: Body + FromBodyStructure + Send + 'static,
    ReqB::Data: Send + 'static,
    ReqB::Error: std::error::Error + Sync + Send + 'static,
{
    pub async fn with_default_http_client(cmd: &Assault, config: WorkerConfig) -> RelentlessResult<Self> {
        let mut clients = HashMap::new();
        for (name, destination) in cmd.override_destination(&config.destinations) {
            let authority = destination.parse::<http::Uri>()?.authority().unwrap().as_str().to_string(); // TODO
            clients.insert(name.to_string(), DefaultHttpClient::<ReqB, BytesBody>::new(authority).await?);
        }

        Self::new(config, clients)
    }
}
impl<S, ReqB, ResB> Worker<S, ReqB, ResB>
where
    ReqB: Body + FromBodyStructure + Send + 'static,
    ReqB::Data: Send + 'static,
    ReqB::Error: std::error::Error + Sync + Send + 'static,
    ResB: Body + Send + 'static,
    ResB::Data: Send + 'static,
    ResB::Error: std::error::Error + Sync + Send + 'static,
    S: Service<http::Request<ReqB>, Response = http::Response<ResB>> + Send + Sync + 'static,
    RelentlessError: From<S::Error>,
{
    pub fn new(config: WorkerConfig, clients: HashMap<String, S>) -> RelentlessResult<Self> {
        let phantom = std::marker::PhantomData;
        Ok(Self { config, clients, phantom })
    }

    pub async fn assault(self, cases: Vec<Case<S, ReqB, ResB>>, cmd: &Assault) -> RelentlessResult<WorkerOutcome> {
        let Self { config, mut clients, .. } = self;

        let mut processes = Vec::new();
        for case in cases {
            // TODO do not await here
            processes.push((case.testcase.clone(), case.process(&mut clients, cmd, &config).await));
        }

        let mut outcome = Vec::new();
        for (testcase, process) in processes {
            let res = process?; // TODO await here
            let pass = if res.len() == 1 { Status::evaluate(res).await? } else { Compare::evaluate(res).await? };
            outcome.push(CaseOutcome::new(testcase, pass));
        }
        Ok(WorkerOutcome::new(config, outcome))
    }
}

/// TODO document
#[derive(Debug, Clone)]
pub struct Case<S, ReqB, ResB> {
    testcase: Testcase,
    phantom: std::marker::PhantomData<(S, ReqB, ResB)>,
}
impl<S, ReqB, ResB> Case<S, ReqB, ResB> {
    pub fn testcase(&self) -> &Testcase {
        &self.testcase
    }
}
impl<S, ReqB, ResB> Case<S, ReqB, ResB>
where
    ReqB: Body + FromBodyStructure + Send + 'static,
    ReqB::Data: Send + 'static,
    ReqB::Error: std::error::Error + Sync + Send + 'static,
    ResB: Body + Send + 'static,
    ResB::Data: Send + 'static,
    ResB::Error: std::error::Error + Sync + Send + 'static,
    S: Service<http::Request<ReqB>, Response = http::Response<ResB>> + Send + Sync + 'static,
    RelentlessError: From<S::Error>,
{
    pub fn new(testcase: Testcase) -> Self {
        let phantom = std::marker::PhantomData;
        Self { testcase, phantom }
    }

    pub async fn process(
        self,
        clients: &mut HashMap<String, S>,
        cmd: &Assault,
        worker_config: &WorkerConfig,
    ) -> RelentlessResult<Vec<http::Response<ResB>>> {
        let setting = &self.testcase.setting.coalesce(&worker_config.setting);

        let mut requests = Vec::new();
        let destinations = cmd.override_destination(&worker_config.destinations);
        for (name, req) in Self::requests(&destinations, &self.testcase.target, setting)? {
            let client = clients.get_mut(&name).unwrap(); // TODO
            let request = client.call(req);
            requests.push(request)
        }

        let mut responses = Vec::new();
        for request in requests {
            let response = request.await?;
            let (part, body) = response.into_parts();
            responses.push(http::Response::from_parts(part, body));
        }
        Ok(responses)
    }

    pub fn requests(
        destinations: &HashMap<String, String>,
        target: &str,
        setting: &Setting,
    ) -> RelentlessResult<HashMap<String, http::Request<ReqB>>> {
        let Setting { protocol, template, timeout } = setting;
        Ok(destinations
            .iter()
            .map(|(name, destination)| {
                let (method, headers, body) = match protocol.clone() {
                    Some(Protocol::Http(http)) => (http.method, http.header, http.body),
                    None => (None, None, None),
                };
                let destination = destination.parse::<http::Uri>().unwrap();
                let uri = http::uri::Builder::from(destination).path_and_query(target).build().unwrap();
                let mut request = http::Request::builder()
                    .uri(uri)
                    .method(method.unwrap_or(http::Method::GET))
                    .body(ReqB::from_body_structure(body.unwrap_or_default()))
                    .unwrap();
                *request.headers_mut() = headers.unwrap_or_default();
                Ok::<_, HttpError>((name.to_string(), request))
            })
            .collect::<Result<HashMap<_, _>, _>>()?)
    }
}
