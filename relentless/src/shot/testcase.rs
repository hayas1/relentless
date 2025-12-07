use futures::{StreamExt, TryStreamExt};
use semigroup::Semigroup;
use serde::{Deserialize, Serialize};
use tower::{timeout::TimeoutLayer, Layer, Service, ServiceBuilder, ServiceExt};

use crate::shot::{
    contract::{Contract, ContractError, RequestSource, ResponseSink, ServiceError, SignContract},
    destinations::Destinations,
    hierarchy::Hierarchy,
    job::JobSpec,
    profile::Profile,
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

#[derive(Debug, Clone, PartialEq)]
pub struct CaseReport<'a, Q, P> {
    case: &'a Testcase<Q, P>,
    passed: usize,
    // messages: Messages<T>,
    // aggregate: EvaluateAggregator,
}
impl<Q, P> Testcase<Q, P> {
    pub async fn shot<T, S, C>(
        &self,
        services: &Destinations<C::Service>,
        job: &JobSpec,
        suite: &Suite<S, Q, P>,
    ) -> crate::Result<CaseReport<Q, P>>
    where
        T: Clone + Service<C::TransportReq, Response = C::TransportRes> + Send,
        S: SignContract<T, C> + Default,
        C: Contract<T, Sign = S, ReqSource = Q, ResSink = P> + Layer<T>,
        C::Service: Clone + Service<C::Request, Response = C::Response>,
        Q: Clone + Semigroup + RequestSource<C::Request>,
        P: Clone + Semigroup + ResponseSink<Result<C::Response, ServiceError<T, C>>>,
    {
        let buffers =
            if Hierarchy::Testcase.contains(&job.sequential) { 1 } else { self.profile.repeat.times().max(1) };
        let profile = &self.profile.clone().semigroup(suite.profile.clone());
        // let services = futures::stream::iter(transports)
        //     .map(|(name, service)| async move {
        //         let layer = sign_contract
        //             .sign_contract(service.clone(), &profile.request, &profile.response)
        //             .await
        //             .map_err(ContractError::<T, C>::Sign)?;
        //         Ok((name, layer.layer(service.clone())))
        //     })
        //     .buffer_unordered(transports.len().max(1))
        //     .try_collect()
        //     .await
        //     .unwrap_or_else(|_: ContractError<T, C>| todo!());
        let destinations = suite.destinations.iter().map(|(d, u)| (d, (**u).clone())).collect();
        let () = profile.shot::<T, C>(services, &destinations, &self.target).await.unwrap_or_else(|_| todo!());
        Ok(CaseReport { case: self, passed: 1 })
    }
}
