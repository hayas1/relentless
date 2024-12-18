use std::marker::PhantomData;

use futures::{stream, Stream, StreamExt};
use tower::{
    timeout::{error::Elapsed, TimeoutLayer},
    Service, ServiceBuilder, ServiceExt,
};

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
    evaluator::{Evaluator, RequestResult},
    factory::RequestFactory,
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
    P: Configuration + Coalesce + Evaluator<S::Response>,
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
        let mut report = Vec::new();
        for config in configs {
            let worker = Worker::new(self.client.clone());
            report.push(worker.assault(cmd, config).await?); // TODO do not await here, use stream
        }

        Ok(Report::new(report))
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
    P: Configuration + Coalesce + Evaluator<S::Response>,
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
        let mut report = Vec::new();
        for testcase in config.testcases {
            let case = Case::new(self.client.clone());
            let testcase = Coalesced::tuple(testcase, worker_config.coalesce().setting);

            let destinations = worker_config.coalesce().destinations;
            report.push(case.assault(cmd, &destinations, testcase).await?); // TODO do not await here, use stream
        }

        Ok(WorkerReport::new(worker_config, report))
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
    P: Configuration + Coalesce + Evaluator<S::Response>,
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
        let _ = cmd;
        let case = &testcase.coalesce();

        let (passed, messages) = self
            .requests(destinations, case)
            .await?
            .fold((0, Vec::new()), |(p, mut msg), res| async move {
                let pass = case.setting.response.evaluate(res, &mut msg).await;
                (p + pass as usize, msg)
            })
            .await;

        Ok(CaseReport::new(testcase, passed, messages.into_iter().collect()))
    }

    pub async fn requests(
        self,
        destinations: &Destinations<http_serde_priv::Uri>,
        testcase: &Testcase<Q, P>,
    ) -> WrappedResult<impl Stream<Item = Destinations<RequestResult<S::Response>>>> {
        let Testcase { target, setting, .. } = testcase;
        let setting_timeout = setting.timeout;

        let timeout = ServiceBuilder::new()
            .option_layer(setting_timeout.map(TimeoutLayer::new))
            .map_err(Into::<tower::BoxError>::into) // https://github.com/tower-rs/tower/issues/665
            .service(self.client);
        let repeat_buffer = if false { 1 } else { setting.repeat.times() };
        Ok(Self::request_stream(destinations, target, setting)
            .map(move |repeating| {
                let timeout = timeout.clone();
                async move {
                    let destinations = repeating.len();
                    stream::iter(repeating)
                        .map(|(d, req)| {
                            let timeout = timeout.clone();
                            async move {
                                match req {
                                    // TODO Service<Req, Response=RequestResult<S::Response>> (as Layer)
                                    Ok(req) => match timeout.clone().ready().await {
                                        Ok(service) => match service.call(req).await {
                                            Ok(res) => (d, RequestResult::Response(res)),
                                            Err(err) => {
                                                if err.is::<Elapsed>() {
                                                    (
                                                        d,
                                                        RequestResult::Timeout(
                                                            setting_timeout.unwrap_or_else(|| unreachable!()),
                                                        ),
                                                    )
                                                } else {
                                                    (d, RequestResult::RequestError(err))
                                                }
                                            }
                                        },
                                        Err(err) => (d, RequestResult::NoReady(err)),
                                    },
                                    Err(err) => (d, RequestResult::FailToMakeRequest(Wrap::from(err))),
                                }
                            }
                        })
                        .buffer_unordered(destinations)
                        .collect::<Destinations<_>>()
                        .await
                }
            })
            .buffered(repeat_buffer))
    }

    pub fn request_stream(
        destinations: &Destinations<http_serde_priv::Uri>,
        target: &str,
        setting: &Setting<Q, P>,
    ) -> impl Stream<Item = Destinations<Result<Req, Q::Error>>> {
        // TODO remove this clone, use lifetime (impl Stream + 'a)
        let (destinations, target, setting) = (destinations.clone(), target.to_string(), setting.clone());
        let Setting { request, template, repeat, .. } = setting;

        stream::iter(repeat.range())
            .map(move |_| {
                let (destinations, target, request, template) =
                    (destinations.clone(), target.to_string(), request.clone(), template.clone());
                async move {
                    destinations
                        .iter()
                        .map(|(name, destination)| {
                            let default_empty = Template::new();
                            let template = template.get(name).unwrap_or(&default_empty);
                            (name.to_string(), request.produce(destination, &target, template))
                        })
                        .collect()
                }
            })
            .buffered(repeat.times())
    }
}
