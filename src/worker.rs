use std::marker::PhantomData;

#[cfg(feature = "default-http-client")]
use crate::service::DefaultHttpClient;
use crate::{
    command::Relentless,
    config::{
        destinations::{Destinations, Transpose},
        http_serde_priv, Coalesced, Config, Setting, Testcase,
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
pub struct Control<'a, S, Req, E> {
    client: &'a mut S,
    evaluator: &'a E,
    phantom: PhantomData<(S, Req, E)>,
}
#[cfg(feature = "default-http-client")]
impl Control<'_, DefaultHttpClient<reqwest::Body, reqwest::Body>, reqwest::Body, DefaultEvaluator> {
    pub async fn default_http_client() -> WrappedResult<DefaultHttpClient<reqwest::Body, reqwest::Body>> {
        DefaultHttpClient::new().await
    }
}
impl<'a, S, Req, E> Control<'a, S, Req, E>
where
    Req: FromRequestInfo,
    S: Service<Req> + Send + 'static,
    S::Error: std::error::Error + Send + Sync + 'static,
    S::Future: Send + 'static,
    E: Evaluator<S::Response>,
    Wrap: From<Req::Error> + From<S::Error>,
{
    /// TODO document
    pub fn new(client: &'a mut S, evaluator: &'a E) -> Self {
        Self { client, evaluator, phantom: PhantomData }
    }
    /// TODO document
    pub async fn assault(self, cmd: &Relentless, configs: Vec<Config>) -> WrappedResult<Report<E::Message>> {
        let mut report = Vec::new();
        for config in configs {
            let worker = Worker::new(self.client, self.evaluator);
            report.push(worker.assault(cmd, config).await?);
        }

        Ok(Report::new(report))
    }
}

/// TODO document
#[derive(Debug)]
pub struct Worker<'a, S, Req, E> {
    client: &'a mut S,
    evaluator: &'a E,
    phantom: PhantomData<(Req, S, E)>,
}
impl<'a, S, Req, E> Worker<'a, S, Req, E>
where
    Req: FromRequestInfo,
    S: Service<Req> + Send + 'static,
    S::Error: std::error::Error + Send + Sync + 'static,
    S::Future: Send + 'static,
    E: Evaluator<S::Response>,
    Wrap: From<Req::Error> + From<S::Error>,
{
    pub fn new(client: &'a mut S, evaluator: &'a E) -> Self {
        Self { client, evaluator, phantom: PhantomData }
    }

    pub async fn assault(self, cmd: &Relentless, config: Config) -> WrappedResult<WorkerReport<E::Message>> {
        let worker_config = Coalesced::tuple(config.worker_config, cmd.destinations()?);
        let mut report = Vec::new();
        for testcase in config.testcases {
            let case = Case::new(self.client);
            let testcase = Coalesced::tuple(testcase, worker_config.coalesce().setting);

            let destinations = worker_config.coalesce().destinations;
            // TODO do not await here, use stream
            let responses = case.process(&destinations, testcase.coalesce()).await?;

            let (mut passed, mut v) = (0, Vec::new());
            for res in responses.transpose() {
                let pass = self.evaluator.evaluate(&testcase.coalesce().setting.evaluate, res, &mut v).await;
                passed += pass as usize;
            }
            report.push(CaseReport::new(testcase, passed, v.into_iter().collect()));
        }

        Ok(WorkerReport::new(worker_config, report))
    }
}

/// TODO document
#[derive(Debug)]
pub struct Case<'a, S, Req> {
    client: &'a mut S,
    phantom: PhantomData<(S, Req)>,
}
impl<'a, S, Req> Case<'a, S, Req>
where
    Req: FromRequestInfo,
    S: Service<Req> + Send + 'static,
    S::Error: std::error::Error + Send + Sync + 'static,
    S::Future: Send + 'static,
    Wrap: From<Req::Error> + From<S::Error>,
{
    pub fn new(client: &'a mut S) -> Self {
        Self { client, phantom: PhantomData }
    }

    pub async fn process(
        self,
        destinations: &Destinations<http_serde_priv::Uri>,
        testcase: Testcase,
    ) -> WrappedResult<Destinations<Vec<RequestResult<S::Response>>>> {
        let Testcase { target, setting, .. } = testcase;

        let mut dest = Destinations::new();
        let mut timeout = ServiceBuilder::new()
            .option_layer(setting.timeout.map(TimeoutLayer::new))
            .map_err(Into::<tower::BoxError>::into) // https://github.com/tower-rs/tower/issues/665
            .service(self.client);
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
        setting: &Setting,
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
