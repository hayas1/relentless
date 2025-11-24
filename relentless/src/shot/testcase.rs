use std::{convert::Infallible, ops::Range, time::Duration};

use futures::{StreamExt, TryStreamExt};
use serde::{Deserialize, Serialize};
use tower::{timeout::TimeoutLayer, Layer, Service, ServiceBuilder, ServiceExt};

use crate::shot::{
    contract::{Contract, RequestSource, ResponseSink, ServiceError, ShotError, SignContract},
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
pub struct CaseReport<Q, P, E> {
    case: Testcase<Q, P>,
    passed: usize,
    // messages: Messages<T>,
    // aggregate: EvaluateAggregator,
    errors: Vec<E>,
}
impl<Q, P> Testcase<Q, P> {
    pub async fn shot<T, S, C>(
        self,
        services: &Destinations<T>,
        sign_contract: &S,
        job: &JobSpec,
        suite: &Suite<Q, P>,
    ) -> crate::Result<CaseReport<Q, P, ShotError<T, C>>>
    where
        T: Clone + Service<C::TransportReq, Response = C::TransportRes> + Send,
        S: SignContract<T, Q, P, C, C::SignError>,
        C: Contract<T, ReqSource = Q, ResSink = P> + Layer<T>,
        C::Service: Service<C::Request, Response = C::Response>,
        Q: RequestSource<C::Request>,
        P: ResponseSink<Result<C::Response, ServiceError<T, C>>>,
    {
        let buffers =
            if Hierarchy::Testcase.contains(&job.sequential) { 1 } else { self.profile.repeat.times().max(1) };
        let (sign_contract_ref, target_ref) = (&sign_contract, &self.target);
        let profile = &self.profile;
        let result = futures::stream::iter(services.iter())
            .map(move |(name, service)| {
                let (sign_contract, target) = (sign_contract_ref, target_ref);
                async move {
                    let layer = sign_contract
                        .sign_contract(service.clone(), &profile.request, &profile.response)
                        .await
                        .map_err(ShotError::<T, C>::Sign)?;
                    let service = layer.layer(service.clone());

                    let destination = suite.destinations.get(name).unwrap_or_else(|| todo!());
                    let request =
                        profile.request.produce(destination, target).await.map_err(ShotError::<T, C>::ReqSource)?;
                    let response = service.oneshot(request).await;
                    Ok((name.clone(), response))
                }
            })
            .buffer_unordered(services.len())
            .try_collect()
            .await;
        match result {
            Ok(responses) => {
                let pass = profile.response.consume(responses).await.map_err(ShotError::<T, C>::ResSink);
                Ok(CaseReport { case: self, passed: pass.is_ok() as usize, errors: pass.err().into_iter().collect() })
            }
            Err(e) => Ok(CaseReport { case: self, passed: 0, errors: vec![e] }),
        }
    }
}
