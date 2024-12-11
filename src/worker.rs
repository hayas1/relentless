use std::marker::PhantomData;

#[cfg(feature = "default-http-client")]
use crate::service::DefaultHttpClient;
use crate::{
    command::Relentless,
    config::{
        destinations::{Destinations, Transpose},
        http_serde_priv, Coalesced, Config, Configuration, HttpRequest, HttpResponse, Setting, Testcase,
    },
    error::{Wrap, WrappedResult},
    evaluate::{DefaultEvaluator, Evaluator, RequestResult},
    report::{CaseReport, Report, WorkerReport},
    service::RequestFactory,
    template::Template,
};
use tower::{
    timeout::{error::Elapsed, TimeoutLayer},
    Service, ServiceBuilder, ServiceExt,
};

/// TODO document
#[derive(Debug)]
pub struct Control<'a, Q, P, S, Req, E> {
    client: &'a mut S,
    evaluator: &'a E,
    phantom: PhantomData<(Q, P, S, Req, E)>,
}
#[cfg(feature = "default-http-client")]
impl
    Control<
        '_,
        HttpRequest,
        HttpResponse,
        DefaultHttpClient<reqwest::Body, reqwest::Body>,
        reqwest::Body,
        DefaultEvaluator,
    >
{
    pub async fn default_http_client() -> WrappedResult<DefaultHttpClient<reqwest::Body, reqwest::Body>> {
        DefaultHttpClient::new().await
    }
}
impl<'a, Q, P, S, Req, E> Control<'a, Q, P, S, Req, E>
where
    Q: Configuration + RequestFactory<Req>,
    P: Configuration,
    S: Service<Req> + Send + 'static,
    S::Error: std::error::Error + Send + Sync + 'static,
    S::Future: Send + 'static,
    E: Evaluator<P, S::Response>,
    Wrap: From<Q::Error> + From<S::Error>,
{
    /// TODO document
    pub fn new(client: &'a mut S, evaluator: &'a E) -> Self {
        Self { client, evaluator, phantom: PhantomData }
    }
    /// TODO document
    pub async fn assault(
        self,
        cmd: &Relentless,
        configs: Vec<Config<Q, P>>,
    ) -> WrappedResult<Report<E::Message, Q, P>> {
        let mut report = Vec::new();
        for config in configs {
            let worker = Worker::new(self.client, self.evaluator);
            report.push(worker.assault(cmd, config).await?); // TODO do not await here, use stream
        }

        Ok(Report::new(report))
    }
}

/// TODO document
#[derive(Debug)]
pub struct Worker<'a, Q, P, S, Req, E> {
    client: &'a mut S,
    evaluator: &'a E,
    phantom: PhantomData<(Q, P, Req, S, E)>,
}
impl<'a, Q, P, S, Req, E> Worker<'a, Q, P, S, Req, E>
where
    Q: Configuration + RequestFactory<Req>,
    P: Configuration,
    S: Service<Req> + Send + 'static,
    S::Error: std::error::Error + Send + Sync + 'static,
    S::Future: Send + 'static,
    E: Evaluator<P, S::Response>,
    Wrap: From<Q::Error> + From<S::Error>,
{
    pub fn new(client: &'a mut S, evaluator: &'a E) -> Self {
        Self { client, evaluator, phantom: PhantomData }
    }

    pub async fn assault(
        self,
        cmd: &Relentless,
        config: Config<Q, P>,
    ) -> WrappedResult<WorkerReport<E::Message, Q, P>> {
        let worker_config = Coalesced::tuple(config.worker_config, cmd.destinations()?);
        let mut report = Vec::new();
        for testcase in config.testcases {
            let case = Case::new(self.client, self.evaluator);
            let testcase = Coalesced::tuple(testcase, worker_config.coalesce().setting);

            let destinations = worker_config.coalesce().destinations;
            report.push(case.assault(cmd, &destinations, testcase).await?); // TODO do not await here, use stream
        }

        Ok(WorkerReport::new(worker_config, report))
    }
}

/// TODO document
#[derive(Debug)]
pub struct Case<'a, Q, P, S, Req, E> {
    client: &'a mut S,
    evaluator: &'a E,
    phantom: PhantomData<(Q, P, S, Req, E)>,
}
impl<'a, Q, P, S, Req, E> Case<'a, Q, P, S, Req, E>
where
    Q: Configuration + RequestFactory<Req>,
    P: Configuration,
    S: Service<Req> + Send + 'static,
    S::Error: std::error::Error + Send + Sync + 'static,
    S::Future: Send + 'static,
    E: Evaluator<P, S::Response>,
    Wrap: From<Q::Error> + From<S::Error>,
{
    pub fn new(client: &'a mut S, evaluator: &'a E) -> Self {
        Self { client, evaluator, phantom: PhantomData }
    }

    pub async fn assault(
        self,
        cmd: &Relentless,
        destinations: &Destinations<http_serde_priv::Uri>,
        testcase: Coalesced<Testcase<Q, P>, Setting<Q, P>>,
    ) -> WrappedResult<CaseReport<E::Message, Q, P>> {
        let _ = cmd;
        let evaluator = self.evaluator;

        // TODO do not await here, use stream
        let responses = self.requests(destinations, testcase.coalesce()).await?;

        let (mut passed, mut v) = (0, Vec::new());
        for res in responses {
            let pass = evaluator.evaluate(&testcase.coalesce().setting.response, res, &mut v).await;
            passed += pass as usize;
        }
        Ok(CaseReport::new(testcase, passed, v.into_iter().collect()))
    }

    pub async fn requests(
        self,
        destinations: &Destinations<http_serde_priv::Uri>,
        testcase: Testcase<Q, P>,
    ) -> WrappedResult<Vec<Destinations<RequestResult<S::Response>>>> {
        let Testcase { target, setting, .. } = testcase;

        let mut repeated = Vec::new();
        let mut timeout = ServiceBuilder::new()
            .option_layer(setting.timeout.map(TimeoutLayer::new))
            .map_err(Into::<tower::BoxError>::into) // https://github.com/tower-rs/tower/issues/665
            .service(self.client);
        for repeating in Self::setup_requests(destinations, &target, &setting)?.transpose() {
            let mut responses = Destinations::new();
            for (d, req) in repeating {
                // TODO do not await here, use stream
                let result = timeout.ready().await.map_err(Wrap::new)?.call(req).await;
                match result {
                    Ok(res) => responses.insert(d, RequestResult::Response(res)),
                    Err(err) => {
                        if err.is::<Elapsed>() {
                            responses
                                .insert(d, RequestResult::Timeout(setting.timeout.unwrap_or_else(|| unreachable!())))
                        } else {
                            Err(Wrap::new(err))?
                        }
                    }
                };
            }
            repeated.push(responses);
        }
        Ok(repeated)
    }

    pub fn setup_requests(
        destinations: &Destinations<http_serde_priv::Uri>,
        target: &str,
        setting: &Setting<Q, P>,
    ) -> WrappedResult<Destinations<Vec<Req>>> {
        let Setting { request, template, repeat, .. } = setting;

        destinations
            .iter()
            .map(|(name, destination)| {
                let default_empty = Template::new();
                let template = template.get(name).unwrap_or(&default_empty);
                let requests = repeat
                    .range()
                    .map(|_| request.produce(destination, target, template))
                    .collect::<Result<Vec<_>, _>>()?;
                Ok((name.to_string(), requests))
            })
            .collect()
    }
}
