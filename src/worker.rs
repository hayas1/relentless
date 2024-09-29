use std::collections::HashMap;

use crate::{
    command::Relentless,
    config::{Config, Protocol, Setting, Testcase, WorkerConfig},
    error::{HttpError, RelentlessError, RelentlessResult},
    outcome::{CaseOutcome, Compare, Evaluator, Outcome, Status, WorkerOutcome},
    service::{BytesBody, DefaultHttpClient, FromBodyStructure},
};
use hyper::body::Body;
use tower::Service;

/// TODO document
#[derive(Debug, Clone)]
pub struct Control<S = DefaultHttpClient<BytesBody, BytesBody>, ReqB = BytesBody, ResB = BytesBody> {
    workers: Vec<Worker<S, ReqB, ResB>>, // TODO all worker do not have same clients type ?
    cases: Vec<Vec<Case<S, ReqB, ResB>>>,
    phantom: std::marker::PhantomData<(ReqB, ResB)>,
}
impl Control<DefaultHttpClient<BytesBody, BytesBody>, BytesBody, BytesBody> {
    pub async fn default_http_clients(
        cmd: &Relentless,
        configs: &Vec<Config>,
    ) -> RelentlessResult<Vec<HashMap<String, DefaultHttpClient<BytesBody, BytesBody>>>> {
        let mut clients = Vec::new();
        for c in configs {
            let mut destinations = HashMap::new();
            for (name, destination) in cmd.override_destination(&c.worker_config.destinations) {
                let authority = destination.parse::<http::Uri>()?.authority().unwrap().as_str().to_string(); // TODO
                let client = DefaultHttpClient::<BytesBody, BytesBody>::new(authority).await?;
                destinations.insert(name.to_string(), client);
            }
            clients.push(destinations);
        }
        Ok(clients)
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
    pub fn with_service(configs: Vec<Config>, services: Vec<HashMap<String, S>>) -> RelentlessResult<Self> {
        let mut workers = Vec::new();
        for (config, service) in configs.iter().zip(services) {
            workers.push(Worker::new(config.worker_config.clone(), service)?);
        }
        Ok(Self::new(configs, workers))
    }
    /// TODO document
    pub fn new(configs: Vec<Config>, workers: Vec<Worker<S, ReqB, ResB>>) -> Self {
        let cases = configs.iter().map(|c| c.testcase.clone().into_iter().map(Case::new).collect()).collect();
        let phantom = std::marker::PhantomData;
        Self { workers, cases, phantom }
    }
    /// TODO document
    pub async fn assault(self, cmd: &Relentless) -> RelentlessResult<Outcome> {
        let Self { workers, cases, .. } = self;

        let mut works = Vec::new();
        for (worker, cases) in workers.into_iter().zip(cases.into_iter()) {
            works.push(worker.assault(cmd, cases));
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

    pub async fn assault(self, cmd: &Relentless, cases: Vec<Case<S, ReqB, ResB>>) -> RelentlessResult<WorkerOutcome> {
        let Self { config, mut clients, .. } = self;

        let mut processes = Vec::new();
        for case in cases {
            // TODO do not await here
            processes.push((case.testcase.clone(), case.process(cmd, &config, &mut clients).await));
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
        cmd: &Relentless,
        worker_config: &WorkerConfig,
        clients: &mut HashMap<String, S>,
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
        let Setting { protocol, template, repeat, timeout } = setting;
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
