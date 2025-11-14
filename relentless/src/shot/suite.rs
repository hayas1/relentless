use futures::{StreamExt, TryStreamExt};
use serde::{Deserialize, Serialize};

use crate::{
    http_newtype_serde,
    shot::{
        destinations::Destinations,
        hierarchy::Hierarchy,
        job::Job,
        testcase::{CaseReport, Profile, Testcase},
    },
};

#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "kebab-case")]
pub struct SuiteCase<Q, P> {
    #[serde(flatten, default)]
    pub suite: Suite<Q, P>,

    #[serde(default)]
    pub testcases: Vec<Testcase<Q, P>>,
}

#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "kebab-case")]
pub struct Suite<Q, P> {
    #[serde(default)]
    pub name: Option<String>,
    #[serde(default)]
    pub destinations: Destinations<http_newtype_serde::Uri>,
    #[serde(default)]
    pub profile: Profile<Q, P>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct SuiteReport<Q, P> {
    cases: Vec<CaseReport<Q, P>>,
}
impl<Q, P> SuiteCase<Q, P> {
    pub async fn shot<S>(self, service: S, job: &Job) -> crate::Result<SuiteReport<Q, P>>
    where
        S: Clone + Send + 'static,
        Q: Send + Sync + 'static,
        P: Send + Sync + 'static,
    {
        let buffers = if Hierarchy::Suite.contains(&job.sequential) { 1 } else { self.testcases.len().max(1) };
        let cases = futures::stream::iter(self.testcases)
            .map(|testcase| testcase.shot(service.clone(), job, &self.suite))
            .buffer_unordered(buffers)
            .try_collect()
            .await?;
        Ok(SuiteReport { cases })
    }
}
