use std::marker::PhantomData;

#[cfg(feature = "default-http-client")]
use crate::service::DefaultHttpClient;
use crate::{
    command::Relentless,
    config::{
        destinations::{Destinations, Transpose},
        http_serde_priv, Coalesced, Config, Configuration, HttpEvaluate, HttpRequest, Setting, Testcase, WorkerConfig,
    },
    error::{Wrap, WrappedResult},
    evaluate::{DefaultEvaluator, Evaluator, RequestResult},
    report::{CaseReport, Report, WorkerReport},
    service::FromRequestInfo,
    template::Template,
};
use tower::{
    timeout::{error::Elapsed, TimeoutLayer},
    Service, ServiceBuilder, ServiceExt,
};

/// TODO document
#[derive(Debug)]
pub struct Control<'a, S, RI, EC, Req, E> {
    _cmd: &'a Relentless,
    workers: Vec<Worker<'a, S, RI, EC, Req, E>>, // TODO all worker do not have same clients type ?
    cases: Vec<Vec<Case<S, RI, EC, Req>>>,
    client: &'a mut S,
}
#[cfg(feature = "default-http-client")]
impl
    Control<
        '_,
        DefaultHttpClient<reqwest::Body, reqwest::Body>,
        HttpRequest,
        HttpEvaluate,
        reqwest::Body,
        DefaultEvaluator,
    >
{
    pub async fn default_http_client() -> WrappedResult<DefaultHttpClient<reqwest::Body, reqwest::Body>> {
        DefaultHttpClient::new().await
    }
}
impl<'a, S, RI, EC, Req, E> Control<'a, S, RI, EC, Req, E>
where
    Req: FromRequestInfo,
    S: Service<Req> + Send + 'static,
    S::Error: std::error::Error + Send + Sync + 'static,
    S::Future: Send + 'static,
    RI: Configuration,
    EC: Configuration,
    E: Evaluator<S::Response>,
    Wrap: From<Req::Error> + From<S::Error>,
{
    /// TODO document
    pub fn with_service(cmd: &'a Relentless, configs: Vec<Config<RI, EC>>, service: &'a mut S) -> WrappedResult<Self> {
        let mut workers = Vec::new();
        for config in &configs {
            workers.push(Worker::new(cmd, config.worker_config.clone())?);
        }
        Ok(Self::new(cmd, configs, workers, service))
    }
    /// TODO document
    pub fn new(
        cmd: &'a Relentless,
        configs: Vec<Config<RI, EC>>,
        workers: Vec<Worker<'a, S, RI, EC, Req, E>>,
        client: &'a mut S,
    ) -> Self {
        let cases = configs
            .iter()
            .map(|c| c.testcases.clone().into_iter().map(|t| Case::new(&c.worker_config, t)).collect())
            .collect();
        Self { _cmd: cmd, workers, cases, client }
    }
    /// TODO document
    pub async fn assault(self, evaluator: &E) -> WrappedResult<Report<E::Message, RI, EC>> {
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
pub struct Worker<'a, S, RI, EC, Req, E> {
    _cmd: &'a Relentless,
    config: Coalesced<WorkerConfig<RI, EC>, Destinations<http_serde_priv::Uri>>,
    phantom: PhantomData<(Req, S, E)>,
}
impl<S, RI, EC, Req, E> Worker<'_, RI, EC, S, Req, E>
where
    RI: Configuration,
    EC: Configuration,
{
    pub fn config(&self) -> WorkerConfig<RI, EC> {
        self.config.coalesce()
    }
}
impl<'a, S, RI, EC, Req, E> Worker<'a, S, RI, EC, Req, E>
where
    Req: FromRequestInfo,
    S: Service<Req> + Send + 'static,
    S::Error: std::error::Error + Send + Sync + 'static,
    S::Future: Send + 'static,
    RI: Configuration,
    EC: Configuration,
    E: Evaluator<S::Response>,
    Wrap: From<Req::Error> + From<S::Error>,
{
    pub fn new(cmd: &'a Relentless, config: WorkerConfig<RI, EC>) -> WrappedResult<Self> {
        let config = Coalesced::tuple(config, cmd.destinations()?);
        let phantom = PhantomData;
        Ok(Self { _cmd: cmd, config, phantom })
    }

    pub async fn assault(
        self,
        cases: Vec<Case<S, RI, EC, Req>>,
        evaluator: &E,
        client: &mut S,
    ) -> WrappedResult<WorkerReport<E::Message, RI, EC>> {
        let Self { config, .. } = self;

        let mut processes = Vec::new();
        for case in cases {
            // TODO do not await here, use stream
            let destinations = config.coalesce().destinations;
            processes.push((case.testcases.clone(), case.process(&destinations, client).await));
        }

        let mut report = Vec::new();
        for (testcase, process) in processes {
            let Setting { evaluate, .. } = &testcase.coalesce().setting;
            let (mut passed, mut v) = (0, Vec::new());
            for res in process?.transpose() {
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
pub struct Case<S, RI, EC, Req> {
    testcases: Coalesced<Testcase<RI, EC>, Setting<RI, EC>>,
    phantom: PhantomData<(S, Req)>,
}
impl<S, RI, EC, Req> Case<S, RI, EC, Req>
where
    RI: Configuration,
    EC: Configuration,
{
    pub fn testcase(&self) -> &Testcase<RI, EC> {
        self.testcases.base()
    }
}
impl<S, RI, EC, Req> Case<S, RI, EC, Req>
where
    Req: FromRequestInfo,
    S: Service<Req> + Send + 'static,
    S::Error: std::error::Error + Send + Sync + 'static,
    S::Future: Send + 'static,
    RI: Configuration,
    EC: Configuration,
    Wrap: From<Req::Error> + From<S::Error>,
{
    pub fn new(worker_config: &WorkerConfig<RI, EC>, testcases: Testcase<RI, EC>) -> Self {
        let testcase = Coalesced::tuple(testcases, worker_config.setting.clone());
        let phantom = PhantomData;
        Self { testcases: testcase, phantom }
    }

    pub async fn process(
        self,
        destinations: &Destinations<http_serde_priv::Uri>,
        client: &mut S,
    ) -> WrappedResult<Destinations<Vec<RequestResult<S::Response>>>> {
        let Testcase { target, setting, .. } = self.testcases.coalesce();

        let mut dest = Destinations::new();
        let mut timeout = ServiceBuilder::new()
            .option_layer(setting.timeout.map(TimeoutLayer::new))
            .map_err(Into::<tower::BoxError>::into) // https://github.com/tower-rs/tower/issues/665
            .service(client);
        for (name, repeated) in Self::requests(destinations, &target, &setting)? {
            let mut responses = Vec::new();
            for req in repeated {
                let result = timeout.ready().await.map_err(Wrap::new)?.call(req).await;
                match result {
                    Ok(res) => responses.push(RequestResult::Response(res)),
                    Err(err) => {
                        if err.is::<Elapsed>() {
                            responses.push(RequestResult::Timeout(setting.timeout.unwrap_or_else(|| unreachable!())));
                        } else {
                            Err(Wrap::new(err))?;
                        }
                    }
                }
            }
            dest.insert(name, responses);
        }
        Ok(dest)
    }

    pub fn requests(
        destinations: &Destinations<http_serde_priv::Uri>,
        target: &str,
        setting: &Setting<RI, EC>,
    ) -> WrappedResult<Destinations<Vec<Req>>> {
        let Setting { request, template, repeat, .. } = setting;

        destinations
            .iter()
            .map(|(name, destination)| {
                let default_empty = Template::new();
                let template = template.get(name).unwrap_or(&default_empty);
                let requests = repeat
                    .range()
                    .map(|_| Req::from_request_info(template, destination, target, request))
                    .collect::<Result<Vec<_>, _>>()?;
                Ok((name.to_string(), requests))
            })
            .collect()
    }
}
