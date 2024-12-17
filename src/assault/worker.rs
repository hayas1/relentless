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
            transpose::Transpose,
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

        // TODO do not await here, use stream
        let responses: Vec<Destinations<_>> = self.requests(destinations, testcase.coalesce()).await.collect().await;

        let (mut passed, mut v) = (0, Vec::new());
        for res in responses {
            let pass = testcase.coalesce().setting.response.evaluate(res, &mut v).await;
            passed += pass as usize;
        }
        Ok(CaseReport::new(testcase, passed, v.into_iter().collect()))
    }

    pub async fn requests(
        self,
        destinations: &Destinations<http_serde_priv::Uri>,
        testcase: Testcase<Q, P>,
    ) -> impl Stream<Item = Destinations<RequestResult<S::Response>>> {
        let Testcase { target, setting, .. } = testcase;
        let setting_timeout = setting.timeout;

        // let mut repeated = Vec::new();
        let timeout = ServiceBuilder::new()
            .option_layer(setting_timeout.map(TimeoutLayer::new))
            .map_err(Into::<tower::BoxError>::into) // https://github.com/tower-rs/tower/issues/665
            .service(self.client);
        stream::iter(Self::setup_requests(destinations, &target, &setting).unwrap_or_else(|_| todo!()).transpose())
            .map(move |repeating| {
                // let (timeout, setting_timeout) = (timeout.clone(), setting_timeout.clone());
                let timeout = timeout.clone();
                async move {
                    let destinations = repeating.len();
                    stream::iter(repeating)
                        .map(|(d, req)| {
                            // let (timeout, setting_timeout) = (timeout.clone(), setting_timeout.clone());
                            let timeout = timeout.clone();
                            async move {
                                let result = timeout.clone().ready().await.map_err(Wrap::new)?.call(req).await;
                                match result {
                                    Ok(res) => Ok((d, RequestResult::Response(res))),
                                    Err(err) => {
                                        if err.is::<Elapsed>() {
                                            Ok((
                                                d,
                                                RequestResult::Timeout(
                                                    setting_timeout.unwrap_or_else(|| unreachable!()),
                                                ),
                                            ))
                                        } else {
                                            Err(Wrap::new(err))?
                                        }
                                    }
                                }
                            }
                        })
                        .buffer_unordered(destinations)
                        .collect::<Vec<WrappedResult<(String, RequestResult<S::Response>)>>>()
                        .await
                        .into_iter()
                        .collect::<WrappedResult<Destinations<RequestResult<S::Response>>>>()
                        .unwrap()
                }
            })
            .buffered(32)
        // for repeating in Self::setup_requests(destinations, &target, &setting)?.transpose() {
        //     let mut responses = Destinations::new();
        //     for (d, req) in repeating {
        //         // TODO do not await here, use stream
        //         let result = timeout.ready().await.map_err(Wrap::new)?.call(req).await;
        //         match result {
        //             Ok(res) => responses.insert(d, RequestResult::Response(res)),
        //             Err(err) => {
        //                 if err.is::<Elapsed>() {
        //                     responses
        //                         .insert(d, RequestResult::Timeout(setting.timeout.unwrap_or_else(|| unreachable!())))
        //                 } else {
        //                     Err(Wrap::new(err))?
        //                 }
        //             }
        //         };
        //     }
        //     repeated.push(responses);
        // }
        // Ok(repeated)
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
