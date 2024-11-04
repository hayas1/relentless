use std::marker::PhantomData;

#[cfg(feature = "default-http-client")]
use crate::service::DefaultHttpClient;
use crate::{
    command::Relentless,
    config::{http_serde_priv, Coalesced, Config, Destinations, RequestInfo, Setting, Testcase, WorkerConfig},
    error::{Wrap, WrappedResult},
    evaluate::{DefaultEvaluator, Evaluator},
    report::{CaseReport, Report, WorkerReport},
    service::FromBodyStructure,
};
use http_body::Body;
use tower::{Service, ServiceExt};

/// TODO document
#[derive(Debug)]
pub struct Control<'a, S, ReqB, ResB, E> {
    _cmd: &'a Relentless,
    workers: Vec<Worker<'a, S, ReqB, ResB, E>>, // TODO all worker do not have same clients type ?
    cases: Vec<Vec<Case<S, ReqB, ResB>>>,
    client: &'a mut S,
    phantom: PhantomData<(ReqB, ResB)>,
}
#[cfg(feature = "default-http-client")]
impl Control<'_, DefaultHttpClient<reqwest::Body, reqwest::Body>, reqwest::Body, reqwest::Body, DefaultEvaluator> {
    pub async fn default_http_client() -> WrappedResult<DefaultHttpClient<reqwest::Body, reqwest::Body>> {
        DefaultHttpClient::new().await
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
    S: Service<http::Request<ReqB>, Response = http::Response<ResB>> + Send + 'static,
    Wrap: From<S::Error>,
    E: Evaluator<http::Response<ResB>>,
{
    /// TODO document
    pub fn with_service(cmd: &'a Relentless, configs: Vec<Config>, service: &'a mut S) -> WrappedResult<Self> {
        let mut workers = Vec::new();
        for config in &configs {
            workers.push(Worker::new(cmd, config.worker_config.clone())?);
        }
        Ok(Self::new(cmd, configs, workers, service))
    }
    /// TODO document
    pub fn new(
        cmd: &'a Relentless,
        configs: Vec<Config>,
        workers: Vec<Worker<'a, S, ReqB, ResB, E>>,
        client: &'a mut S,
    ) -> Self {
        let cases = configs
            .iter()
            .map(|c| c.testcases.clone().into_iter().map(|t| Case::new(&c.worker_config, t)).collect())
            .collect();
        let phantom = PhantomData;
        Self { _cmd: cmd, workers, cases, phantom, client }
    }
    /// TODO document
    pub async fn assault(self, evaluator: &E) -> WrappedResult<Report<E::Message>> {
        let Self { workers, cases, .. } = self;

        let mut report = Vec::new();
        for (worker, cases) in workers.into_iter().zip(cases.into_iter()) {
            report.push(worker.assault(cases, evaluator, self.client).await?);
        }

        Ok(Report::new(report))
    }
}

/// TODO document
#[derive(Debug)]
pub struct Worker<'a, S, ReqB, ResB, E> {
    _cmd: &'a Relentless,
    config: Coalesced<WorkerConfig, Destinations<http_serde_priv::Uri>>,
    phantom: PhantomData<(ReqB, ResB, S, E)>,
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
    S: Service<http::Request<ReqB>, Response = http::Response<ResB>> + Send + 'static,
    Wrap: From<S::Error>,
    E: Evaluator<http::Response<ResB>>,
{
    pub fn new(cmd: &'a Relentless, config: WorkerConfig) -> WrappedResult<Self> {
        let config = Coalesced::tuple(config, cmd.destinations()?);
        let phantom = PhantomData;
        Ok(Self { _cmd: cmd, config, phantom })
    }

    pub async fn assault(
        self,
        cases: Vec<Case<S, ReqB, ResB>>,
        evaluator: &E,
        client: &mut S,
    ) -> WrappedResult<WorkerReport<E::Message>> {
        let Self { config, .. } = self;

        let mut processes = Vec::new();
        for case in cases {
            // TODO do not await here, use stream
            let destinations = config.coalesce().destinations;
            processes.push((case.testcases.clone(), case.process(&destinations, client).await));
        }

        let mut report = Vec::new();
        for (testcase, process) in processes {
            let Testcase { setting, .. } = testcase.coalesce();
            let Setting { repeat, evaluate, .. } = &setting;
            let mut passed = 0;
            let mut t = repeat.range().map(|_| Destinations::new()).collect::<Vec<_>>();
            for (name, repeated) in process? {
                for (i, res) in repeated.into_iter().enumerate() {
                    t[i].insert(name.clone(), res);
                }
            }
            let mut v = Vec::new();
            for res in t {
                let pass = evaluator.evaluate(evaluate, res, &mut v).await;
                passed += pass as usize;
            }

            report.push(CaseReport::new(testcase, passed, v.into_iter().collect()));
        }
        Ok(WorkerReport::new(config, report))
    }
}

/// TODO document
#[derive(Debug, Clone)]
pub struct Case<S, ReqB, ResB> {
    testcases: Coalesced<Testcase, Setting>,
    phantom: PhantomData<(S, ReqB, ResB)>,
}
impl<S, ReqB, ResB> Case<S, ReqB, ResB> {
    pub fn testcase(&self) -> &Testcase {
        self.testcases.base()
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
    S: Service<http::Request<ReqB>, Response = http::Response<ResB>> + Send + 'static,
    Wrap: From<S::Error>,
{
    pub fn new(worker_config: &WorkerConfig, testcases: Testcase) -> Self {
        let testcase = Coalesced::tuple(testcases, worker_config.setting.clone());
        let phantom = PhantomData;
        Self { testcases: testcase, phantom }
    }

    pub async fn process(
        self,
        destinations: &Destinations<http_serde_priv::Uri>,
        client: &mut S,
    ) -> WrappedResult<Destinations<Vec<http::Response<ResB>>>> {
        let Testcase { target, setting, .. } = self.testcases.coalesce();

        let mut dest = Destinations::new();
        for (name, repeated) in Self::requests(destinations, &target, &setting)? {
            let mut responses = Vec::new();
            for req in repeated {
                let res = client.ready().await?.call(req).await?;
                responses.push(res);
            }
            dest.insert(name, responses);
        }
        Ok(dest)
    }

    pub fn requests(
        destinations: &Destinations<http_serde_priv::Uri>,
        target: &str,
        setting: &Setting,
    ) -> WrappedResult<Destinations<Vec<http::Request<ReqB>>>> {
        let Setting { request, template, repeat, timeout, .. } = setting;

        if !template.is_empty() {
            unimplemented!("template is not implemented yet");
        }
        if timeout.is_some() {
            unimplemented!("timeout is not implemented yet");
        }

        destinations
            .iter()
            .map(|(name, destination)| {
                let requests = repeat
                    .range()
                    .map(|_| Self::http_request(destination, target, request))
                    .collect::<Result<Vec<_>, _>>()?;
                Ok((name.to_string(), requests))
            })
            .collect()
    }

    // TODO generics
    pub fn http_request(
        destination: &http::Uri,
        target: &str,
        request_info: &RequestInfo,
    ) -> WrappedResult<http::Request<ReqB>> {
        let RequestInfo { no_additional_headers, method, headers, body } = &request_info;
        let uri = http::uri::Builder::from(destination.clone()).path_and_query(target).build()?;
        let applied_method = method.as_ref().map(|m| (**m).clone()).unwrap_or_default();
        let assigned_headers = headers.as_ref().map(|h| (**h).clone()).unwrap_or_default();
        let (actual_body, additional_headers) = body.clone().unwrap_or_default().body_with_headers()?;

        let mut request = http::Request::builder().uri(uri).method(applied_method).body(actual_body)?;
        let header_map = request.headers_mut();
        header_map.extend(assigned_headers);
        if !no_additional_headers {
            header_map.extend(additional_headers);
        }
        Ok(request)
    }
}
