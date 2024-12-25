use std::marker::PhantomData;

use futures::{stream, Stream, StreamExt, TryStreamExt};
use tower::{timeout::TimeoutLayer, Service, ServiceBuilder, ServiceExt};

use crate::{
    assault::reportable::{CaseReport, Report, WorkerReport},
    error::{Wrap, WrappedResult},
    interface::{
        command::Relentless,
        config::{Config, Configuration, Setting, Testcase},
        helper::{
            coalesce::{Coalesce, Coalesced},
            http_serde_priv,
        },
        template::Template,
    },
};

use super::{
    destinations::Destinations,
    error::{RequestError, RequestResult},
    evaluate::Evaluate,
    factory::RequestFactory,
    measure::aggregate::{Aggregate, EvaluateAggregator},
    messages::Messages,
    service::measure::MeasureLayer,
};

/// TODO document
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Control<Q, P, S, Req> {
    client: S,
    phantom: PhantomData<(Q, P, S, Req)>,
}
impl<Q, P, S, Req> Control<Q, P, S, Req>
where
    Q: Configuration + Coalesce + RequestFactory<Req>,
    Q::Error: std::error::Error + Send + Sync + 'static,
    P: Configuration + Coalesce + Evaluate<S::Response>,
    S: Service<Req> + Clone + Send + 'static,
    S::Error: std::error::Error + Send + Sync + 'static,
    S::Future: Send + 'static,
    Wrap: From<Q::Error> + From<S::Error>,
{
    /// TODO document
    pub fn new(client: S) -> Self {
        Self { client, phantom: PhantomData }
    }
    /// TODO document
    pub async fn assault(
        self,
        cmd: &Relentless,
        configs: Vec<Config<Q, P>>,
    ) -> WrappedResult<Report<P::Message, Q, P>> {
        let configs_buffer = if cmd.no_async_configs { 1 } else { configs.len() };

        let report = stream::iter(configs)
            .map(|config| {
                let worker = Worker::new(self.client.clone());
                async move { worker.assault(cmd, config).await }
            })
            .buffer_unordered(configs_buffer)
            .try_collect()
            .await;

        Ok(Report::new(report?))
    }
}

/// TODO document
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Worker<Q, P, S, Req> {
    client: S,
    phantom: PhantomData<(Q, P, Req, S)>,
}
impl<Q, P, S, Req> Worker<Q, P, S, Req>
where
    Q: Configuration + Coalesce + RequestFactory<Req>,
    Q::Error: std::error::Error + Send + Sync + 'static,
    P: Configuration + Coalesce + Evaluate<S::Response>,
    S: Service<Req> + Clone + Send + 'static,
    S::Error: std::error::Error + Send + Sync + 'static,
    S::Future: Send + 'static,
    Wrap: From<Q::Error> + From<S::Error>,
{
    pub fn new(client: S) -> Self {
        Self { client, phantom: PhantomData }
    }

    pub async fn assault(
        self,
        cmd: &Relentless,
        config: Config<Q, P>,
    ) -> WrappedResult<WorkerReport<P::Message, Q, P>> {
        let worker_config = Coalesced::tuple(config.worker_config, cmd.destinations()?);
        let testcase_buffer = if cmd.no_async_testcases { 1 } else { config.testcases.len() };

        let report = stream::iter(config.testcases)
            .map(|testcase| {
                let case = Case::new(self.client.clone());
                let testcase = Coalesced::tuple(testcase, worker_config.coalesce().setting);
                let destinations = worker_config.coalesce().destinations;
                async move { case.assault(cmd, &destinations, testcase).await }
            })
            .buffered(testcase_buffer)
            .try_collect()
            .await;

        Ok(WorkerReport::new(worker_config, report?))
    }
}

/// TODO document
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Case<Q, P, S, Req> {
    client: S,
    phantom: PhantomData<(Q, P, S, Req)>,
}
impl<Q, P, S, Req> Case<Q, P, S, Req>
where
    Q: Configuration + Coalesce + RequestFactory<Req>,
    Q::Error: std::error::Error + Send + Sync + 'static,
    P: Configuration + Coalesce + Evaluate<S::Response>,
    S: Service<Req> + Clone + Send + 'static,
    S::Error: std::error::Error + Send + Sync + 'static,
    S::Future: Send + 'static,
    Wrap: From<Q::Error> + From<S::Error>,
{
    pub fn new(client: S) -> Self {
        Self { client, phantom: PhantomData }
    }

    pub async fn assault(
        self,
        cmd: &Relentless,
        destinations: &Destinations<http_serde_priv::Uri>,
        testcase: Coalesced<Testcase<Q, P>, Setting<Q, P>>,
    ) -> WrappedResult<CaseReport<P::Message, Q, P>> {
        let case = &testcase.coalesce();
        let evaluate_aggregate = EvaluateAggregator::new(destinations, None);

        let (passed, messages, aggregate) = self
            .requests(cmd, destinations, case)
            .await?
            .fold((0, Messages::new(), evaluate_aggregate), |(p, mut msg, mut agg), res| async move {
                let metrics = res.iter().map(|(d, r)| (d, r.as_ref().ok().map(|r| r.metrics().clone()))).collect();
                let pass = case.setting.response.evaluate(res, &mut msg).await;
                agg.add(&(pass, metrics)); // TODO timeout request will be not measured
                (p + pass as usize, msg, agg)
            })
            .await;

        Ok(CaseReport::new(testcase, passed, messages, aggregate))
    }

    pub async fn requests<'a>(
        self,
        cmd: &Relentless,
        destinations: &'a Destinations<http_serde_priv::Uri>,
        testcase: &'a Testcase<Q, P>,
    ) -> WrappedResult<impl Stream<Item = Destinations<RequestResult<S::Response>>> + 'a> {
        let Testcase { target, setting, .. } = testcase;
        let client = self.client.clone();

        let repeat_buffer = if cmd.no_async_repeat { 1 } else { setting.repeat.times() };
        Ok(Self::request_stream(destinations, target, setting)
            .map(move |repeating| {
                let client = client.clone();
                async move {
                    let destinations = repeating.len();
                    stream::iter(repeating)
                        .map(|(d, req)| {
                            let client = client.clone();
                            async move { (d, Self::call(client, req, setting).await) }
                        })
                        .buffer_unordered(destinations)
                        .collect()
                        .await
                }
            })
            .buffered(repeat_buffer))
    }

    pub async fn call(client: S, req: Result<Req, Q::Error>, setting: &Setting<Q, P>) -> RequestResult<S::Response> {
        let mut service = ServiceBuilder::new()
            .layer(MeasureLayer)
            .map_err(RequestError::InnerServiceError) // TODO how to handle this error?
            .option_layer(setting.timeout.map(TimeoutLayer::new))
            .map_err(Into::<tower::BoxError>::into) // https://github.com/tower-rs/tower/issues/665
            .service(client.clone());

        let request = req.map_err(|e| RequestError::FailToMakeRequest(e.into()))?;
        service.ready().await.map_err(|e| RequestError::NoReady(e.into()))?.call(request).await
    }

    pub fn request_stream<'a>(
        destinations: &'a Destinations<http_serde_priv::Uri>,
        target: &'a str,
        setting: &'a Setting<Q, P>,
    ) -> impl Stream<Item = Destinations<Result<Req, Q::Error>>> + 'a {
        let Setting { request, template, repeat, .. } = setting;

        stream::iter(repeat.range())
            .map(move |_| async move {
                destinations
                    .iter()
                    .map(|(name, destination)| {
                        let default_empty = Template::new();
                        let template = template.get(name).unwrap_or(&default_empty);
                        (name.to_string(), request.produce(destination, target, template))
                    })
                    .collect()
            })
            .buffer_unordered(repeat.times())
    }
}
