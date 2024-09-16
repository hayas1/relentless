use std::{collections::HashMap, time::Duration};

use crate::{
    config::{Protocol, Setting, Testcase, WorkerConfig},
    error::{CaseError, HttpError, RelentlessError, RelentlessResult},
    outcome::{CaseOutcome, Compare, Evaluator, Status, WorkerOutcome},
};
use http::Method;
use reqwest::{Client, Request, Response};
use tokio::task::JoinSet;
use tower::Service;

#[derive(Debug, Clone)]
pub enum CaseService {
    Http(Case<Client>),
}

#[derive(Debug, Clone)]
pub struct Case<S> {
    testcase: Testcase,
    client: S,
}
impl Case<Client> {
    pub fn new_http(testcase: Testcase) -> Self {
        let client = Client::new();
        Self::new(testcase, client)
    }
}
impl<S> Case<S>
where
    S: Clone + Service<Request, Response = Response> + Send + 'static,
    S::Future: Send + 'static,
    S::Error: Send + 'static,
    RelentlessError: From<S::Error>,
{
    pub fn new(testcase: Testcase, client: S) -> Self {
        Self { testcase, client }
    }

    pub async fn process(&self, worker_config: &WorkerConfig) -> RelentlessResult<Vec<Response>> {
        let mut join_set = JoinSet::<RelentlessResult<Response>>::new();
        for (name, req) in
            Self::requests(&self.testcase.target, &self.testcase.setting.coalesce(&worker_config.setting))?
        {
            for _ in 0..self.testcase.attr.repeat.unwrap_or(1) {
                let r = req.try_clone().ok_or(CaseError::FailCloneRequest)?;
                let mut client = self.client.clone();
                join_set.spawn(async move {
                    let res = client.call(r).await?;
                    Ok(res)
                });
            }
        }

        let mut response = Vec::new();
        while let Some(res) = join_set.join_next().await {
            response.push(res??);
        }
        Ok(response)
    }

    pub fn requests(target: &str, setting: &Setting) -> RelentlessResult<HashMap<String, Request>> {
        let Setting { protocol, origin, template, timeout } = setting;
        Ok(origin
            .iter()
            .map(|(name, origin)| {
                let (method, headers, body) = match protocol.clone() {
                    Some(Protocol::Http(http)) => (http.method, http.header, http.body),
                    None => (None, None, None),
                };
                let url = reqwest::Url::parse(origin)?.join(target)?;
                let mut request = Request::new(method.unwrap_or(Method::GET), url);
                *request.timeout_mut() = timeout.or(Some(Duration::from_secs(10)));
                *request.headers_mut() = headers.unwrap_or_default();
                *request.body_mut() = body.map(|b| b.into());
                Ok::<_, HttpError>((name.clone(), request))
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

    pub async fn assault(self, cases: Vec<CaseService>) -> RelentlessResult<WorkerOutcome> {
        let mut outcome = Vec::new();
        for case in cases {
            match case {
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
