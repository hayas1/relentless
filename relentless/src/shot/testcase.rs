use std::{ops::Range, time::Duration};

use futures::{StreamExt, TryStreamExt};
use serde::{Deserialize, Serialize};
use tower::{timeout::TimeoutLayer, Service, ServiceBuilder, ServiceExt};

use crate::shot::{
    contract::{Contract, RequestSource},
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
    profile: Profile<Q, P>,
    passed: usize,
    // messages: Messages<T>,
    // aggregate: EvaluateAggregator,
}
impl<Q, P> Testcase<Q, P> {
    pub async fn shot<S, C>(
        self,
        services: &Destinations<S>,
        job: &JobSpec,
        suite: &Suite<Q, P>,
    ) -> crate::Result<CaseReport<Q, P>>
    where
        S: Clone + Service<C::Request, Response = C::Response> + Send,
        C: Contract<S, ReqSource = Q, ResSink = P>,
        C::Service: for<'x> Service<RequestSource<'x, C::ReqSource>>,
        Q: Send + Sync + 'static,
        P: Send + Sync + 'static,
    {
        let buffers =
            if Hierarchy::Testcase.contains(&job.sequential) { 1 } else { self.profile.repeat.times().max(1) };
        let profile = &self.profile;
        let target = &self.target.clone();
        let result: Destinations<_> = futures::stream::iter(services.iter())
            .map(|(name, service)| async move {
                let layer = C::new(service.clone(), &profile.request).await.unwrap_or_else(|_| todo!());
                let service = layer.layer(service.clone());

                let destination = suite.destinations.get(name).unwrap_or_else(|| todo!());
                let request = RequestSource { destination, target, source: &profile.request };
                let response = service.oneshot(request).await?;
                Ok::<_, <C::Service as Service<RequestSource<'_, C::ReqSource>>>::Error>((name.clone(), response))
            })
            .buffer_unordered(services.len())
            .try_collect()
            .await
            .unwrap_or_else(|_| todo!());
        Ok(CaseReport { profile: todo!(), passed: 0 })
    }
}
