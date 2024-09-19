use std::{collections::HashMap, time::Duration};

use crate::{
    config::{Protocol, Setting, Testcase, WorkerConfig},
    error::{HttpError, RelentlessError, RelentlessResult},
    outcome::{CaseOutcome, Compare, Evaluator, Status, WorkerOutcome},
};
use http::Method;
use tokio::task::JoinSet;
use tower::Service;

#[derive(Debug)]
pub enum CaseService<S, Req, Res> {
    Default(Case<S, Req, Res>),
    Http(Case<reqwest::Client, reqwest::Request, reqwest::Response>),
}
#[derive(Debug)]
pub enum CaseRequest<Req> {
    Default(Req),
    Http(reqwest::Request),
}
#[derive(Debug)]
pub enum CaseResponse<Res> {
    Default(Res),
    Http(reqwest::Response),
}

#[derive(Debug, Clone)]
pub struct Case<S, Req, Res> {
    testcase: Testcase,
    clients: HashMap<String, S>,
    phantom: std::marker::PhantomData<(Req, Res)>,
}
impl Case<reqwest::Client, reqwest::Request, reqwest::Response> {
    pub fn new_http(testcase: Testcase) -> Self {
        let clients = testcase.setting.origin.keys().map(|name| (name.clone(), reqwest::Client::new())).collect();
        Self::new(testcase, clients)
    }
}
impl<S, Req, Res> Case<S, Req, Res>
where
    Req: Send + 'static,
    Res: Send + 'static,
    S: Clone + Service<http::Request<Req>, Response = http::Response<Res>> + Send + 'static,
    S::Future: Send + 'static,
    S::Error: Send + 'static,
    RelentlessError: From<S::Error>,
{
    pub fn new(testcase: Testcase, clients: HashMap<String, S>) -> Self {
        let phantom = std::marker::PhantomData;
        Self { testcase, clients, phantom }
    }

    pub async fn process(&self, worker_config: &WorkerConfig) -> RelentlessResult<Vec<CaseResponse<Res>>> {
        let mut join_set = JoinSet::<RelentlessResult<CaseResponse<Res>>>::new();
        for (name, req) in
            Self::requests(&self.testcase.target, &self.testcase.setting.coalesce(&worker_config.setting))?
        {
            // for _ in 0..self.testcase.attr.repeat.unwrap_or(1) {
            let r = req; //.clone(); //.ok_or(CaseError::FailCloneRequest)?;
            let mut client = self.clients.clone();
            join_set.spawn(async move {
                match r {
                    CaseRequest::Default(r) => {
                        todo!()
                    }
                    CaseRequest::Http(req) => {
                        // let mut client = self.clients[&name]; // TODO
                        let mut client = reqwest::Client::new();
                        let res = client.call(req).await?;
                        Ok(CaseResponse::Http(res))
                    }
                }
            });
            // }
        }

        let mut response = Vec::new();
        while let Some(res) = join_set.join_next().await {
            response.push(res??);
        }
        Ok(response)
    }

    pub fn requests(target: &str, setting: &Setting) -> RelentlessResult<HashMap<String, CaseRequest<Req>>> {
        let Setting { protocol, origin, template, timeout } = setting;
        Ok(origin
            .iter()
            .map(|(name, origin)| {
                let (method, headers, body) = match protocol.clone() {
                    Some(Protocol::Http(http)) => (http.method, http.header, http.body),
                    None => (None, None, None),
                };
                let url = reqwest::Url::parse(origin)?.join(target)?;
                let mut request = reqwest::Request::new(method.unwrap_or(Method::GET), url);
                *request.timeout_mut() = timeout.or(Some(Duration::from_secs(10)));
                *request.headers_mut() = headers.unwrap_or_default();
                *request.body_mut() = body.map(|b| b.into());
                Ok::<_, HttpError>((name.clone(), CaseRequest::Http(request)))
            })
            .collect::<Result<HashMap<_, _>, _>>()?)
    }
}

#[derive(Debug, Clone)]
pub struct Worker {
    config: WorkerConfig,
}
impl Worker {
    pub fn new(config: WorkerConfig) -> Self {
        Self { config }
    }

    pub async fn assault<S, Req, Res>(self, cases: Vec<CaseService<S, Req, Res>>) -> RelentlessResult<WorkerOutcome>
    where
        Req: Send + 'static,
        Res: Send + 'static,
        S: Clone + Service<http::Request<Req>, Response = http::Response<Res>> + Send + 'static,
        S::Future: Send + 'static,
        S::Error: Send + 'static,
        RelentlessError: From<S::Error>,
    {
        let mut outcome = Vec::new();
        for c in cases {
            match c {
                CaseService::Default(case) => {
                    let res = case.process(&self.config).await?;
                    let pass =
                        if res.len() == 1 { Status::evaluate(res).await? } else { Compare::evaluate(res).await? };
                    outcome.push(CaseOutcome::new(case.testcase, pass));
                }
                CaseService::Http(case) => {
                    let res = case.process(&self.config).await?;
                    let pass =
                        if res.len() == 1 { Status::evaluate(res).await? } else { Compare::evaluate(res).await? };
                    outcome.push(CaseOutcome::new(case.testcase, pass));
                }
            };
        }
        Ok(WorkerOutcome::new(self.config, outcome))
    }
}
