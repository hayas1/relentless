use std::{collections::HashMap, time::Duration};

use crate::{
    config::{Protocol, Setting, Testcase, WorkerConfig},
    error::{CaseError, HttpError, RelentlessError, RelentlessResult},
    outcome::{CaseOutcome, Compare, Evaluator, Status, WorkerOutcome},
};
use http::Method;
use reqwest::{Client, Request, Response};
use tokio::task::JoinSet;
use tower::{Layer, Service};

#[derive(Debug)]
pub struct Case<LC> {
    testcase: Testcase,
    layer: Option<LC>,
}
impl<LC> Case<LC> {
    pub fn new(testcase: Testcase, layer: Option<LC>) -> Self {
        Self { testcase, layer }
    }

    pub async fn process<LW>(&self, layer: Option<LW>, worker_config: &WorkerConfig) -> RelentlessResult<Vec<Response>>
    where
        LC: Layer<Client> + Clone + Send + 'static,
        LC::Service: Service<Request> + Send,
        <LC::Service as Service<Request>>::Future: Send,
        <LC::Service as Service<Request>>::Response: Into<Response> + Send + 'static,
        <LC::Service as Service<Request>>::Error: Send + 'static,
        LW: Layer<Client> + Clone + Send + 'static,
        LW::Service: Service<Request> + Send,
        <LW::Service as Service<Request>>::Future: Send,
        <LW::Service as Service<Request>>::Response: Into<Response> + Send + 'static,
        <LW::Service as Service<Request>>::Error: Send + 'static,
        RelentlessError:
            From<<LC::Service as Service<Request>>::Error> + From<<LW::Service as Service<Request>>::Error>,
    {
        let mut join_set = JoinSet::<RelentlessResult<Response>>::new();
        for (name, req) in
            Self::requests(&self.testcase.target, &self.testcase.setting.coalesce(&worker_config.setting))?
        {
            for _ in 0..self.testcase.attr.repeat.unwrap_or(1) {
                let r = req.try_clone().ok_or(CaseError::FailCloneRequest)?;
                let mut client = Client::new();
                let (case_layer, worker_layer) = (self.layer.clone(), layer.clone());
                join_set.spawn(async move {
                    let res = match case_layer {
                        Some(layer) => layer.layer(client).call(r).await?.into(),
                        None => match worker_layer {
                            Some(layer) => layer.layer(client).call(r).await?.into(),
                            None => client.call(r).await?,
                        },
                    };
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

pub struct Worker<LW> {
    config: WorkerConfig,
    layer: Option<LW>,
}
impl<LW> Worker<LW> {
    pub fn new(config: WorkerConfig, layer: Option<LW>) -> Self {
        Self { config, layer }
    }

    pub async fn assault<LC>(self, cases: Vec<Case<LC>>) -> RelentlessResult<WorkerOutcome>
    where
        LW: Layer<Client> + Clone + Send + 'static,
        LW::Service: Service<Request> + Send,
        <LW::Service as Service<Request>>::Future: Send,
        <LW::Service as Service<Request>>::Response: Into<Response> + Send + 'static,
        <LW::Service as Service<Request>>::Error: Send + 'static,
        LC: Layer<Client> + Clone + Send + 'static,
        LC::Service: Service<Request> + Send,
        <LC::Service as Service<Request>>::Future: Send,
        <LC::Service as Service<Request>>::Response: Into<Response> + Send + 'static,
        <LC::Service as Service<Request>>::Error: Send + 'static,
        RelentlessError:
            From<<LW::Service as Service<Request>>::Error> + From<<LC::Service as Service<Request>>::Error>,
    {
        let mut outcome = Vec::new();
        for case in cases {
            let res = case.process(self.layer.clone(), &self.config).await?;
            let pass = if res.len() == 1 { Status::evaluate(res).await? } else { Compare::evaluate(res).await? };
            outcome.push(CaseOutcome::new(case.testcase, pass));
        }
        Ok(WorkerOutcome::new(self.config, outcome))
    }
}
