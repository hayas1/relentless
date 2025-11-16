use std::{
    future::Future,
    pin::Pin,
    task::{Context, Poll},
};

use futures::{StreamExt, TryStreamExt};
use serde::{Deserialize, Serialize};
use tower::{Service, ServiceExt};

use crate::{
    generator::Generator,
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
impl<Q, P> Suite<Q, P> {
    pub async fn service<'a, S>(
        &'a self,
        job: &'a Job,
        s: impl AsyncFn(&Self) -> crate::Result<S>,
    ) -> crate::Result<SuiteService<'a, S, Q, P>> {
        Ok(SuiteService { service: s(self).await?, job, suite: self })
    }
}
#[derive(Debug, Clone, PartialEq)]
pub struct SuiteService<'a, S, Q, P> {
    service: S,
    job: &'a Job,
    suite: &'a Suite<Q, P>,
}
impl<'a, S, Q, P> Service<Testcase<Q, P>> for SuiteService<'a, S, Q, P>
where
    S: 'a + Clone + Service<Q::Output> + Send + Sync,
    Q: Generator<S> + Send + Sync + 'static,
    P: Send + Sync + 'static,
{
    type Response = CaseReport<Q, P>;
    type Error = S::Error;
    type Future = Pin<Box<dyn 'a + Future<Output = Result<Self::Response, Self::Error>> + Send>>;

    fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        self.service.poll_ready(cx)
    }

    fn call(&mut self, case: Testcase<Q, P>) -> Self::Future {
        let (service, job, suite) = (self.service.clone(), self.job, self.suite);
        Box::pin(async { Ok(case.shot(service, job, suite).await.unwrap()) })
    }
}
impl<'a, S, Q, P> Service<Testcase<Q, P>> for &'a SuiteService<'a, S, Q, P>
where
    S: 'a + Clone + Service<Q::Output> + Send + Sync,
    Q: Generator<S> + Send + Sync + 'static,
    P: Send + Sync + 'static,
{
    type Response = CaseReport<Q, P>;
    type Error = S::Error;
    type Future = Pin<Box<dyn 'a + Future<Output = Result<Self::Response, Self::Error>> + Send>>;

    fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        self.service.clone().poll_ready(cx)
    }

    fn call(&mut self, case: Testcase<Q, P>) -> Self::Future {
        let (service, job, suite) = (self.service.clone(), self.job, self.suite);
        Box::pin(async { Ok(case.shot(service, job, suite).await.unwrap()) })
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct SuiteReport<Q, P> {
    cases: Vec<CaseReport<Q, P>>,
}
impl<Q, P> SuiteCase<Q, P> {
    pub async fn shot<S>(
        self,
        s: impl AsyncFn(&Suite<Q, P>) -> crate::Result<S>,
        job: &Job,
    ) -> crate::Result<SuiteReport<Q, P>>
    where
        S: Clone + Service<Q::Output> + Send + Sync + 'static,
        Q: Generator<S> + Send + Sync + 'static,
        P: Send + Sync + 'static,
    {
        let buffers = if Hierarchy::Suite.contains(&job.sequential) { 1 } else { self.testcases.len().max(1) };
        let suite = &self.suite.service(job, s).await?;
        let cases = futures::stream::iter(self.testcases)
            .map(|testcase| suite.oneshot(testcase))
            .buffer_unordered(buffers)
            .try_collect()
            .await
            .unwrap_or_else(|_| todo!());
        Ok(SuiteReport { cases })
    }
}
