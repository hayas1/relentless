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
        client::Client,
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
    pub async fn service<'a, S: Client<S, Generator = Q, Evaluator = P>>(
        &'a self,
        job: &'a Job,
    ) -> crate::Result<SuiteService<'a, S, Q, P>> {
        let mut services = Destinations::default();
        for (d, dest) in self.destinations.iter() {
            services.insert(d.to_string(), S::connect(dest, &self.profile).await.unwrap_or_else(|_| todo!()));
        }

        Ok(SuiteService { services, job, suite: self })
    }
}
#[derive(Debug, Clone, PartialEq)]
pub struct SuiteService<'a, S, Q, P> {
    services: Destinations<S>,
    job: &'a Job,
    suite: &'a Suite<Q, P>,
}
impl<'a, S, Q, P> Service<Testcase<Q, P>> for SuiteService<'a, S, Q, P>
where
    S: 'a + Clone + Service<Q::Request> + Send + Sync,
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
    S: 'a + Clone + Service<Q::Request> + Send + Sync,
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
    pub async fn shot<S>(self, job: &Job) -> crate::Result<SuiteReport<Q, P>>
    where
        S: Clone + Client<S, Generator = Q, Evaluator = P> + Service<Q::Request> + Send + Sync + 'static,
        Q: Generator<S> + Send + Sync + 'static,
        P: Send + Sync + 'static,
    {
        let buffers = if Hierarchy::Suite.contains(&job.sequential) { 1 } else { self.testcases.len().max(1) };
        let suite = &self.suite.service(job).await?;
        let cases = futures::stream::iter(self.testcases)
            .map(|testcase| suite.oneshot(testcase))
            .buffer_unordered(buffers)
            .try_collect()
            .await
            .unwrap_or_else(|_| todo!());
        Ok(SuiteReport { cases })
    }
}
