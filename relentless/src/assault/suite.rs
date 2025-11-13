use std::{
    future::Future,
    pin::Pin,
    sync::Arc,
    task::{Context, Poll},
};

use futures::{StreamExt, TryStreamExt};
use serde::{Deserialize, Serialize};
use tower::{Service, ServiceExt};

use crate::{
    assault::{
        destinations::Destinations,
        hierarchy::Hierarchy,
        job::Job,
        testcase::{CaseReport, Profile, Testcase},
    },
    http_newtype_serde,
};

#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "kebab-case")]
pub struct SuiteCases<Q, P> {
    #[serde(flatten, default)]
    pub suite: Suite<Q, P>,

    #[serde(default)]
    pub testcases: Vec<Testcase<Q, P>>,
}
impl<Q, P> SuiteCases<Q, P> {
    pub async fn assault<S>(self, service: S, job: Arc<Job>) -> crate::Result<SuiteReport<Q, P>>
    where
        S: Clone + Send + 'static,
        Q: Send + Sync + 'static,
        P: Send + Sync + 'static,
    {
        let suite = Arc::new(self.suite);
        let buffers = if Hierarchy::Suite.contains(&job.sequential) { 1 } else { self.testcases.len().max(1) };
        let cases = futures::stream::iter(self.testcases)
            .map(|testcase| SuiteService::new(job.clone(), suite.clone(), service.clone()).oneshot(testcase))
            .buffer_unordered(buffers)
            .try_collect()
            .await?;
        Ok(SuiteReport { cases })
    }
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

#[derive(Debug, Clone, PartialEq)]
pub struct SuiteService<S, Q, P> {
    job: Arc<Job>,
    suite: Arc<Suite<Q, P>>,
    service: S,
}
impl<S, Q, P> SuiteService<S, Q, P> {
    pub fn new(job: Arc<Job>, suite: Arc<Suite<Q, P>>, service: S) -> Self {
        Self { job, suite, service }
    }
}

impl<S, Q, P> Service<Testcase<Q, P>> for SuiteService<S, Q, P>
where
    S: Clone + Send + 'static,
    Q: Send + Sync + 'static,
    P: Send + Sync + 'static,
{
    type Response = CaseReport<Q, P>;
    type Error = crate::Error;
    type Future = Pin<Box<dyn Future<Output = Result<Self::Response, Self::Error>> + Send>>;

    fn poll_ready(&mut self, _: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        Poll::Ready(Ok(()))
    }

    fn call(&mut self, testcase: Testcase<Q, P>) -> Self::Future {
        let (job, suite, service) = (self.job.clone(), self.suite.clone(), self.service.clone());
        Box::pin(async move { testcase.assault(service, job, suite) })
    }
}
