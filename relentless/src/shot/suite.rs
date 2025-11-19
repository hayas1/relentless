use std::{
    future::Future,
    pin::Pin,
    task::{Context, Poll},
};

use futures::{StreamExt, TryStreamExt};
use serde::{Deserialize, Serialize};
use tower::{MakeService, Service, ServiceExt};

use crate::{
    generator::Generator,
    http_newtype_serde,
    shot::{
        destinations::Destinations,
        hierarchy::Hierarchy,
        job::JobSpec,
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
    pub async fn service<'a, M, S>(
        &'a self,
        make_service: M,
        job: &'a JobSpec,
    ) -> crate::Result<SuiteService<'a, M::Service, Q, P>>
    where
        M: Clone + MakeService<http::Uri, Q::Request, Service = S>,
        S: Clone + Service<Q::Request> + Send,
        Q: Generator<S>,
    {
        let mut services = Destinations::default();
        for (d, http_newtype_serde::Uri(dest)) in self.destinations.iter() {
            services.insert(
                d.to_string(),
                make_service.clone().make_service(dest.clone()).await.unwrap_or_else(|_| todo!()),
            );
        }

        Ok(SuiteService { services, job, suite: self })
    }
}
#[derive(Debug, Clone, PartialEq)]
pub struct SuiteService<'a, S, Q, P> {
    services: Destinations<S>,
    job: &'a JobSpec,
    suite: &'a Suite<Q, P>,
}
impl<'a, S, Q, P> Service<Testcase<Q, P>> for SuiteService<'a, S, Q, P>
where
    S: 'a + Clone + Service<Q::Request> + Send,
    Q: Generator<S> + Send + Sync + 'static,
    P: Send + Sync + 'static,
{
    type Response = CaseReport<Q, P>;
    type Error = S::Error;
    type Future = Pin<Box<dyn 'a + Future<Output = Result<Self::Response, Self::Error>> + Send>>;

    fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        match self
            .services
            .values_mut()
            .try_fold(true, |and, s| Ok(and && matches!(s.poll_ready(cx)?, Poll::Ready(()))))
        {
            Ok(true) => Poll::Ready(Ok(())),
            Ok(false) => Poll::Pending,
            Err(e) => Poll::Ready(Err(e)),
        }
    }

    fn call(&mut self, case: Testcase<Q, P>) -> Self::Future {
        let (service, job, suite) = (self.services.clone(), self.job, self.suite);
        Box::pin(async { Ok(case.shot(service, job, suite).await.unwrap()) })
    }
}
impl<'a, S, Q, P> Service<Testcase<Q, P>> for &'a SuiteService<'a, S, Q, P>
where
    S: 'a + Clone + Service<Q::Request> + Send,
    Q: Generator<S> + Send + Sync + 'static,
    P: Send + Sync + 'static,
{
    type Response = CaseReport<Q, P>;
    type Error = S::Error;
    type Future = Pin<Box<dyn 'a + Future<Output = Result<Self::Response, Self::Error>> + Send>>;

    fn poll_ready(&mut self, _cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        Poll::Ready(Ok(()))
    }

    fn call(&mut self, case: Testcase<Q, P>) -> Self::Future {
        let (service, job, suite) = (self.services.clone(), self.job, self.suite);
        Box::pin(async { Ok(case.shot(service, job, suite).await.unwrap()) })
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct SuiteReport<Q, P> {
    cases: Vec<CaseReport<Q, P>>,
}
impl<Q, P> SuiteCase<Q, P> {
    pub async fn shot<M, S>(self, make_service: M, job: &JobSpec) -> crate::Result<SuiteReport<Q, P>>
    where
        M: Clone + MakeService<http::Uri, Q::Request, Service = S>,
        S: Clone + Service<Q::Request> + Send,
        Q: Generator<S> + Send + Sync + 'static,
        P: Send + Sync + 'static,
    {
        let buffers = if Hierarchy::Suite.contains(&job.sequential) { 1 } else { self.testcases.len().max(1) };
        let suite = &self.suite.service(make_service, job).await?;
        let cases = futures::stream::iter(self.testcases)
            .map(|testcase| suite.oneshot(testcase))
            .buffer_unordered(buffers)
            .try_collect()
            .await
            .unwrap_or_else(|_| todo!());
        Ok(SuiteReport { cases })
    }
}
