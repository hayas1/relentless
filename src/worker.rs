use std::{collections::HashMap, time::Duration};

use crate::{
    config::{BodyStructure, FromBodyStructure, Protocol, Setting, Testcase, WorkerConfig},
    error::{HttpError, RelentlessError, RelentlessResult},
    outcome::{CaseOutcome, Compare, Evaluator, Status, WorkerOutcome},
    service::HyperClient,
};
use bytes::Bytes;
use http::{Method, Response};
use http_body_util::Empty;
use hyper::body::{Body, Incoming};
use tokio::{runtime::Runtime, task::JoinSet};
use tower::Service;

#[derive(Debug, Clone)]
pub struct Worker<S, ReqB, ResB> {
    config: WorkerConfig,
    clients: HashMap<String, S>,
    phantom: std::marker::PhantomData<(ReqB, ResB)>,
}
impl<ReqB, ResB> Worker<HyperClient<ReqB, ResB>, ReqB, ResB>
where
    ReqB: Body + FromBodyStructure + Send + 'static,
    ReqB::Data: Send + 'static,
    ReqB::Error: std::error::Error + Sync + Send + 'static,
    ResB: From<Bytes> + Send + Sync + 'static,
{
    pub async fn with_hyper_client(config: WorkerConfig) -> RelentlessResult<Self> {
        let mut clients = HashMap::new();
        for (name, origin) in &config.origins {
            let host = origin.parse::<http::Uri>()?.authority().unwrap().as_str().to_string(); // TODO
            clients.insert(name.clone(), HyperClient::<ReqB, ResB>::new(host).await?);
        }

        Self::new(config, clients)
    }
}
impl<S, ReqB, ResB> Worker<S, ReqB, ResB>
where
    ReqB: Body + FromBodyStructure + Send + 'static,
    ReqB::Data: Send + 'static,
    ReqB::Error: std::error::Error + Sync + Send + 'static,
    ResB: From<Bytes> + Send + 'static,
    S: Clone + Service<http::Request<ReqB>, Response = http::Response<ResB>> + Send + Sync + 'static,
    RelentlessError: From<S::Error>,
{
    pub fn new(config: WorkerConfig, clients: HashMap<String, S>) -> RelentlessResult<Self> {
        let phantom = std::marker::PhantomData;
        Ok(Self { config, clients, phantom })
    }

    pub async fn assault(self, cases: Vec<CaseService<S, ReqB, ResB>>) -> RelentlessResult<WorkerOutcome> {
        let mut outcome = Vec::new();
        for c in cases {
            match c {
                CaseService::Default(case) => {
                    let mut clients = self.clients.clone();
                    let res = case.process(&mut clients, &self.config).await?;
                    let pass =
                        if res.len() == 1 { Status::evaluate(res).await? } else { Compare::evaluate(res).await? };
                    outcome.push(CaseOutcome::new(case.testcase, pass));
                }
                CaseService::Http(case) => {
                    todo!()
                    // let res = case.process(&mut self.clients, &self.config).await?;
                    // let pass =
                    //     if res.len() == 1 { Status::evaluate(res).await? } else { Compare::evaluate(res).await? };
                    // outcome.push(CaseOutcome::new(case.testcase, pass));
                }
            };
        }
        Ok(WorkerOutcome::new(self.config.clone(), outcome))
    }
}

#[derive(Debug)]
pub enum CaseService<S, ReqB, ResB> {
    Default(Case<S, ReqB, ResB>),
    Http(Case<HyperClient<ReqB, Bytes>, ReqB, Bytes>),
}
#[derive(Debug)]
pub enum CaseRequest<ReqB> {
    Default(ReqB),
    Http(http::Request<ReqB>),
}
#[derive(Debug)]
pub enum CaseResponse<ResB> {
    Default(ResB),
    Http(http::Response<ResB>),
}

#[derive(Debug, Clone)]
pub struct Case<S, ReqB, ResB> {
    testcase: Testcase,
    phantom: std::marker::PhantomData<(S, ReqB, ResB)>,
}
impl<ReqB, ResB> Case<HyperClient<ReqB, ResB>, ReqB, ResB>
where
    ReqB: Body + FromBodyStructure + Send + 'static,
    ReqB::Data: Send + 'static,
    ReqB::Error: std::error::Error + Sync + Send + 'static,
    ResB: From<Bytes> + Send + Sync + 'static,
{
    pub fn new_http(testcase: Testcase) -> Self {
        Self::new(testcase)
    }
}
impl<S, ReqB, ResB> Case<S, ReqB, ResB>
where
    ReqB: Body + FromBodyStructure + Send + 'static,
    ReqB::Data: Send + 'static,
    ReqB::Error: std::error::Error + Sync + Send + 'static,
    ResB: From<Bytes> + Send + 'static,
    S: Clone + Service<http::Request<ReqB>, Response = http::Response<ResB>> + Send + Sync + 'static,
    RelentlessError: From<S::Error>,
{
    pub fn new(testcase: Testcase) -> Self {
        let phantom = std::marker::PhantomData;
        Self { testcase, phantom }
    }

    pub async fn process(
        &self,
        clients: &mut HashMap<String, S>,
        worker_config: &WorkerConfig,
    ) -> RelentlessResult<Vec<CaseResponse<ResB>>> {
        let setting = &self.testcase.setting.coalesce(&worker_config.setting);
        let mut requests = Vec::new();
        for (name, req) in Self::requests(&worker_config.origins, &self.testcase.target, setting)? {
            let r = req;
            match r {
                CaseRequest::Default(r) => {
                    todo!()
                }
                CaseRequest::Http(req) => {
                    let client = clients.get_mut(&name).unwrap(); // TODO
                    let fut = client.call(req);
                    requests.push(fut)
                }
            }
        }

        let mut responses = Vec::new();
        for fut in requests {
            let res = fut.await?;
            let (part, body) = res.into_parts();
            responses.push(CaseResponse::Http(http::Response::from_parts(part, body)));
        }
        Ok(responses)
    }

    pub fn requests(
        origins: &HashMap<String, String>,
        target: &str,
        setting: &Setting,
    ) -> RelentlessResult<HashMap<String, CaseRequest<ReqB>>> {
        let Setting { protocol, template, timeout } = setting;
        Ok(origins
            .iter()
            .map(|(name, origin)| {
                let (method, headers, body) = match protocol.clone() {
                    Some(Protocol::Http(http)) => (http.method, http.header, http.body),
                    None => (None, None, None),
                };
                let uri = http::uri::Builder::new()
                    .scheme("http")
                    .authority("localhost:3000")
                    .path_and_query(target)
                    .build()
                    .unwrap();
                let request = http::Request::builder()
                    .uri(uri)
                    .method(method.unwrap_or(Method::GET))
                    .body(ReqB::from_body_structure(body.unwrap_or_default()))
                    .unwrap();
                Ok::<_, HttpError>((name.clone(), CaseRequest::Http(request)))
            })
            .collect::<Result<HashMap<_, _>, _>>()?)
    }
}
