use futures::StreamExt;
use semigroup::{CombineStream, Semigroup};
use serde::{Deserialize, Serialize};
use tower::{Layer, Service};

use crate::shot::{
    contract::{Contract, Evaluated, RequestSource, ResponseSink, ServiceError, SignContract},
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
    pub case: &'a Testcase<Q, P>,
    pub evaluated: Evaluated,
    // messages: Messages<T>,
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
        let profile = &self.profile.clone().semigroup(suite.profile.clone());
        let buffers = if Hierarchy::Testcase.contains(&job.sequential) { 1 } else { profile.repeat.times().max(1) };

        let destinations = suite.destinations.iter().map(|(d, u)| (d, (**u).clone())).collect();
        let evaluated = futures::stream::iter(profile.repeat.range())
            .map(|_| async {
                let evaluated = profile.shot::<T, C>(services, &destinations, &self.target).await;
                Evaluated::new(evaluated, profile.allow)
            })
            .buffer_unordered(buffers)
            .combine_monoid()
            .await;
        let () = profile.shot::<T, C>(services, &destinations, &self.target).await.unwrap_or_else(|_| todo!());
        Ok(CaseReport { case: self, evaluated })
    }
}
