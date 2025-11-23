use std::{convert::Infallible, ops::Range, time::Duration};

use futures::{StreamExt, TryStreamExt};
use serde::{Deserialize, Serialize};
use tower::{timeout::TimeoutLayer, Layer, Service, ServiceBuilder, ServiceExt};

use crate::shot::{
    contract::{Contract, MakeContract, RequestSource, ResponseSink, ServiceError},
    destinations::Destinations,
    hierarchy::Hierarchy,
    job::JobSpec,
    suite::Suite,
};

#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "kebab-case")]
pub struct Testcase<Q, P> {
    #[serde(default)]
    pub description: Option<String>,
    pub target: String,

    #[serde(default)]
    pub profile: Profile<Q, P>,
}

#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "kebab-case")]
pub struct Profile<Q, P> {
    #[serde(default)]
    pub request: Q,

    // #[serde(default, with = "transpose::transpose_template_serde")]
    // pub template: Destinations<Template>,
    #[serde(default)]
    pub repeat: Repeat,
    #[serde(default)]
    pub timeout: Option<Duration>, // TODO parse from string? https://crates.io/crates/humantime ?
    #[serde(default)]
    pub allow: Option<bool>,

    #[serde(default)]
    pub response: P,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Default, Hash, Serialize, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "kebab-case")]
pub struct Repeat(pub Option<usize>);
impl Repeat {
    pub fn range(&self) -> Range<usize> {
        0..self.times()
    }
    pub fn times(&self) -> usize {
        self.0.unwrap_or(1)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct CaseReport<Q, P> {
    case: Testcase<Q, P>,
    passed: usize,
    // messages: Messages<T>,
    // aggregate: EvaluateAggregator,
}
impl<Q, P> Testcase<Q, P> {
    pub async fn shot<S, N, C>(
        self,
        services: &Destinations<S>,
        make_contract: &N,
        job: &JobSpec,
        suite: &Suite<Q, P>,
    ) -> crate::Result<CaseReport<Q, P>>
    where
        S: Clone + Service<C::TransportReq, Response = C::TransportRes> + Send,
        N: MakeContract<S, Q, P, C, C::MakeError>,
        C: Contract<S, ReqSource = Q, ResSink = P> + Layer<S>,
        C::Service: Service<C::Request, Response = C::Response>,
        Q: RequestSource<C::Request>,
        P: ResponseSink<Result<C::Response, ServiceError<C, S>>>,
    {
        let buffers =
            if Hierarchy::Testcase.contains(&job.sequential) { 1 } else { self.profile.repeat.times().max(1) };
        let (make_contract_ref, target_ref) = (&make_contract, &self.target);
        let profile = &self.profile;
        let result: Destinations<_> = futures::stream::iter(services.iter())
            .map(move |(name, service)| {
                let (make_contract, target) = (make_contract_ref, target_ref);
                async move {
                    let layer = make_contract
                        .make_contract(service.clone(), &profile.request, &profile.response)
                        .await
                        .unwrap_or_else(|_| todo!());
                    let service = layer.layer(service.clone());

                    let destination = suite.destinations.get(name).unwrap_or_else(|| todo!());
                    let request = profile.request.produce(destination, target).await.unwrap_or_else(|_| todo!());
                    let response = service.oneshot(request).await;
                    Ok::<_, Infallible>((name.clone(), response))
                }
            })
            .buffer_unordered(services.len())
            .try_collect()
            .await
            .unwrap_or_else(|_| todo!());
        let pass = profile.response.consume(result).await;
        Ok(CaseReport { case: self, passed: pass.is_ok() as usize })
    }
}
