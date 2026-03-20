use std::fmt::Debug;

use futures::StreamExt;
use semigroup::{CombineStream, Semigroup};
use serde::{Deserialize, Serialize};
use tower::{Layer, Service};

use crate::{
    evaluator::evaluate::Messages,
    shot::{
        contract::{Contract, Evaluated, RequestSource, ResponseSink, ServiceError, SignContract},
        destinations::Destinations,
        hierarchy::Hierarchy,
        job::JobSpec,
        profile::Profile,
        suite::Suite,
    },
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
pub struct CaseReport<'a, Q, P, M> {
    pub case: &'a Testcase<Q, P>,
    pub evaluated: Evaluated,
    pub messages: Messages<M>,
}

impl<Q, P> Testcase<Q, P> {
    #[tracing::instrument(name = "testcase", skip(services))]
    pub async fn shot<T, S, C>(
        &self,
        services: &Destinations<C::Service>,
        destinations: &Destinations<http::Uri>,
        job: &JobSpec,
        suite: &Suite<S, Q, P>,
    ) -> crate::Result<CaseReport<'_, Q, P, P::Message>>
    where
        T: Clone + Service<C::TransportReq, Response = C::TransportRes> + Send,
        S: Debug + SignContract<T, C> + Default,
        C: Contract<T, Sign = S, ReqSource = Q, ResSink = P> + Layer<T>,
        C::Service: Clone + Service<C::Request, Response = C::Response>,
        Q: Debug + Clone + Semigroup + RequestSource<C::Request>,
        P: Debug + Clone + Semigroup + ResponseSink<Result<C::Response, ServiceError<T, C>>>,
    {
        let profile = &self.profile.clone().semigroup(suite.profile.clone());
        let buffers = if Hierarchy::Testcase.contains(&job.sequential) { 1 } else { profile.repeat.times().max(1) };

        let (evaluated, messages) = futures::stream::iter(profile.repeat.range())
            .map(|_| async {
                profile.shot::<T, C>(services, &destinations, &self.target).await.unwrap_or_else(|_| todo!())
            })
            .buffer_unordered(buffers)
            .combine_monoid()
            .await;
        Ok(CaseReport { case: self, evaluated, messages })
    }
}
