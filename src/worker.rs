use std::marker::PhantomData;

#[cfg(feature = "default-http-client")]
use crate::service::DefaultHttpClient;
use crate::{
    command::Relentless,
    config::{Coalesce, Coalesced, Config, Destinations, Protocol, Setting, Testcase, WorkerConfig},
    error::WrappedResult,
    evaluate::{DefaultEvaluator, Evaluator},
    outcome::{CaseOutcome, Outcome, WorkerOutcome},
    service::FromBodyStructure,
};
use http_body::Body;
use tower::{Service, ServiceExt};

/// TODO document
#[derive(Debug, Clone)]
pub struct Control<'a, S, ReqB, ResB, E> {
    _cmd: &'a Relentless,
    workers: Vec<Worker<'a, S, ReqB, ResB, E>>, // TODO all worker do not have same clients type ?
    cases: Vec<Vec<Case<S, ReqB, ResB>>>,
    phantom: PhantomData<(ReqB, ResB)>,
}
#[cfg(feature = "default-http-client")]
impl Control<'_, DefaultHttpClient<reqwest::Body, reqwest::Body>, reqwest::Body, reqwest::Body, DefaultEvaluator> {
    pub async fn default_http_clients(
        cmd: &Relentless,
        configs: &Vec<Config>,
    ) -> WrappedResult<Vec<Destinations<DefaultHttpClient<reqwest::Body, reqwest::Body>>>> {
        let mut clients = Vec::new();
        for c in configs {
            clients.push(Self::default_http_client(cmd, c).await?);
        }
        Ok(clients)
    }
    pub async fn default_http_client(
        cmd: &Relentless,
        config: &Config,
    ) -> WrappedResult<Destinations<DefaultHttpClient<reqwest::Body, reqwest::Body>>> {
        let mut destinations = Destinations::new();
        for (name, _destination) in config.worker_config.destinations.clone().coalesce(&cmd.destination) {
            let client = DefaultHttpClient::<reqwest::Body, reqwest::Body>::new().await?;
            destinations.insert(name.to_string(), client);
        }
        Ok(destinations)
    }
}
impl<'a, S, ReqB, ResB, E> Control<'a, S, ReqB, ResB, E>
where
    ReqB: Body + FromBodyStructure + Send + 'static,
    ReqB::Data: Send + 'static,
    ReqB::Error: std::error::Error + Sync + Send + 'static,
    ResB: Body + Send + 'static,
    ResB::Data: Send + 'static,
    ResB::Error: std::error::Error + Sync + Send + 'static,
    S: Service<http::Request<ReqB>, Response = http::Response<ResB>> + Send + Sync + 'static,
    S::Error: std::error::Error + Sync + Send + 'static,
    E: Evaluator<http::Response<ResB>>,
    E::Error: std::error::Error + Sync + Send + 'static,
{
    /// TODO document
    pub fn with_service(
        cmd: &'a Relentless,
        configs: Vec<Config>,
        services: Vec<Destinations<S>>,
    ) -> WrappedResult<Self> {
        let mut workers = Vec::new();
        for (config, service) in configs.iter().zip(services) {
            workers.push(Worker::new(cmd, config.worker_config.clone(), service)?);
        }
        Ok(Self::new(cmd, configs, workers))
    }
    /// TODO document
    pub fn new(cmd: &'a Relentless, configs: Vec<Config>, workers: Vec<Worker<'a, S, ReqB, ResB, E>>) -> Self {
        let cases = configs
            .iter()
            .map(|c| c.testcase.clone().into_iter().map(|t| Case::new(&c.worker_config, t)).collect())
            .collect();
        let phantom = PhantomData;
        Self { _cmd: cmd, workers, cases, phantom }
    }
    /// TODO document
    pub async fn assault(self, evaluator: &E) -> WrappedResult<Outcome> {
        let Self { workers, cases, .. } = self;

        let mut works = Vec::new();
        for (worker, cases) in workers.into_iter().zip(cases.into_iter()) {
            works.push(worker.assault(cases, evaluator));
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
pub struct Worker<'a, S, ReqB, ResB, E> {
    _cmd: &'a Relentless,
    config: Coalesced<WorkerConfig, Destinations<String>>,
    clients: Destinations<S>,
    phantom: PhantomData<(ReqB, ResB, E)>,
}
impl<S, ReqB, ResB, E> Worker<'_, S, ReqB, ResB, E> {
    pub fn config(&self) -> WorkerConfig {
        self.config.coalesce()
    }
}
impl<'a, S, ReqB, ResB, E> Worker<'a, S, ReqB, ResB, E>
where
    ReqB: Body + FromBodyStructure + Send + 'static,
    ReqB::Data: Send + 'static,
    ReqB::Error: std::error::Error + Sync + Send + 'static,
    ResB: Body + Send + 'static,
    ResB::Data: Send + 'static,
    ResB::Error: std::error::Error + Sync + Send + 'static,
    S: Service<http::Request<ReqB>, Response = http::Response<ResB>> + Send + Sync + 'static,
    S::Error: std::error::Error + Sync + Send + 'static,
    E: Evaluator<http::Response<ResB>>,
    E::Error: std::error::Error + Sync + Send + 'static,
{
    pub fn new(cmd: &'a Relentless, config: WorkerConfig, clients: Destinations<S>) -> WrappedResult<Self> {
        let config = Coalesced::tuple(config, cmd.destination.clone().into_iter().collect());
        let phantom = PhantomData;
        Ok(Self { _cmd: cmd, config, clients, phantom })
    }

    pub async fn assault(self, cases: Vec<Case<S, ReqB, ResB>>, evaluator: &E) -> WrappedResult<WorkerOutcome> {
        let Self { config, mut clients, .. } = self;

        let mut processes = Vec::new();
        for case in cases {
            // TODO do not await here, use stream
            let destinations = config.coalesce().destinations;
            processes.push((case.testcase.clone(), case.process(&destinations, &mut clients).await));
        }

        let mut outcome = Vec::new();
        for (testcase, process) in processes {
            let Testcase { setting, .. } = testcase.coalesce();
            let Setting { repeat, evaluate, .. } = &setting;
            let mut passed = 0;
            let mut t = (0..repeat.unwrap_or(1)).map(|_| Destinations::new()).collect::<Vec<_>>();
            for (name, repeated) in process? {
                for (i, res) in repeated.into_iter().enumerate() {
                    t[i].insert(name.clone(), res);
                }
            }
            for res in t {
                let pass = evaluator.evaluate(evaluate.as_ref(), res).await?;
                passed += pass as usize;
            }
            outcome.push(CaseOutcome::new(testcase, passed));
        }
        Ok(WorkerOutcome::new(config, outcome))
    }
}

/// TODO document
#[derive(Debug, Clone)]
pub struct Case<S, ReqB, ResB> {
    testcase: Coalesced<Testcase, Setting>,
    phantom: PhantomData<(S, ReqB, ResB)>,
}
impl<S, ReqB, ResB> Case<S, ReqB, ResB> {
    pub fn testcase(&self) -> &Testcase {
        self.testcase.base()
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
    S::Error: std::error::Error + Sync + Send + 'static,
{
    pub fn new(worker_config: &WorkerConfig, testcase: Testcase) -> Self {
        let testcase = Coalesced::tuple(testcase, worker_config.setting.clone());
        let phantom = PhantomData;
        Self { testcase, phantom }
    }

    pub async fn process(
        self,
        destinations: &Destinations<String>,
        clients: &mut Destinations<S>,
    ) -> WrappedResult<Destinations<Vec<http::Response<ResB>>>> {
        let Testcase { target, setting, .. } = self.testcase.coalesce();

        let mut dest = Destinations::new();
        for (name, repeated) in Self::requests(destinations, &target, &setting)? {
            let mut responses = Vec::new();
            for req in repeated {
                let client = clients.get_mut(&name).unwrap();
                let res = client.ready().await?.call(req).await?;
                responses.push(res);
            }
            dest.insert(name, responses);
        }
        Ok(dest)
    }

    pub fn requests(
        destinations: &Destinations<String>,
        target: &str,
        setting: &Setting,
    ) -> WrappedResult<Destinations<Vec<http::Request<ReqB>>>> {
        let Setting { protocol, template, repeat, timeout, .. } = setting;

        if !template.is_empty() {
            unimplemented!("template is not implemented yet");
        }
        if timeout.is_some() {
            unimplemented!("timeout is not implemented yet");
        }

        destinations
            .iter()
            .map(|(name, destination)| {
                let default_http = Default::default();
                let http = protocol
                    .as_ref()
                    .map(|p| match p {
                        Protocol::Http(http) => http,
                    })
                    .unwrap_or(&default_http);
                let requests = (0..repeat.unwrap_or(1))
                    .map(|_| Self::http_request(destination, target, http))
                    .collect::<Result<Vec<_>, _>>()
                    .unwrap(); // TODO
                Ok((name.to_string(), requests))
            })
            .collect()
    }

    // TODO generics
    pub fn http_request(
        destination: &str,
        target: &str,
        http: &crate::config::Http,
    ) -> WrappedResult<http::Request<ReqB>> {
        let destination = destination.parse::<http::Uri>().unwrap();
        let uri = http::uri::Builder::from(destination).path_and_query(target).build().unwrap();
        let mut request = http::Request::builder()
            .uri(uri)
            .method(http.method.clone().unwrap_or(http::Method::GET))
            .body(ReqB::from_body_structure(http.body.clone().unwrap_or_default()))
            .unwrap();
        *request.headers_mut() = http.header.clone().unwrap_or_default();
        Ok(request)
    }
}
