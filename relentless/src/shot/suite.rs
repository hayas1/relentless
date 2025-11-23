use std::convert::Infallible;

use futures::{StreamExt, TryStreamExt};
use serde::{Deserialize, Serialize};
use tower::{Layer, MakeService, Service};

use crate::{
    http_newtype_serde,
    shot::{
        contract::{Contract, MakeContract, RequestSource, ResponseSink, ServiceError},
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
// impl<Q, P> Suite<Q, P> {
//     pub async fn service<'a, M, S, C>(
//         &'a self,
//         make_service: M,
//         job: &'a JobSpec,
//     ) -> crate::Result<SuiteService<'a, M::Service, C>>
//     where
//         M: Clone + MakeService<http::Uri, C::Request, Service = S>,
//         S: Clone + Service<C::Request, Response = C::Response> + Send,
//         C: Contract<S, ReqSource = Q, ResSink = P>,
//         C::Service: for<'x> Service<RequestSource<&'x C::ReqSource>>,
//     {
//         let mut services = Destinations::default();
//         for (d, http_newtype_serde::Uri(dest)) in self.destinations.iter() {
//             services.insert(
//                 d.to_string(),
//                 make_service.clone().make_service(dest.clone()).await.unwrap_or_else(|_| todo!()),
//             );
//         }

//         Ok(SuiteService { services, job, suite: self })
//     }
// }
// #[derive(Debug, Clone, PartialEq)]
// pub struct SuiteService<'a, S, C: Contract<S>> {
//     services: Destinations<S>,
//     job: &'a JobSpec,
//     suite: &'a Suite<C::ReqSource, C::ResSink>,
// }
// impl<'a, S, C> Service<Testcase<C::ReqSource, C::ResSink>> for SuiteService<'a, S, C>
// where
//     S: 'a + Clone + Service<C::Request, Response = C::Response> + Send,
//     C: Contract<S>,
//     C::Service: for<'x> Service<RequestSource<&'x C::ReqSource>> + Send,
//     for<'x> <C::Service as Service<RequestSource<&'x C::ReqSource>>>::Future: Send,
//     C::ReqSource: Send + Sync + 'static,
//     C::ResSink: Send + Sync + 'static,
// {
//     type Response = CaseReport<C::ReqSource, C::ResSink>;
//     type Error = S::Error;
//     type Future = Pin<Box<dyn 'a + Future<Output = Result<Self::Response, Self::Error>> + Send>>;

//     fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
//         match self
//             .services
//             .values_mut()
//             .try_fold(true, |and, s| Ok(and && matches!(s.poll_ready(cx)?, Poll::Ready(()))))
//         {
//             Ok(true) => Poll::Ready(Ok(())),
//             Ok(false) => Poll::Pending,
//             Err(e) => Poll::Ready(Err(e)),
//         }
//     }

//     fn call(&mut self, case: Testcase<C::ReqSource, C::ResSink>) -> Self::Future {
//         let (service, job, suite) = (self.services.clone(), self.job, self.suite);
//         Box::pin(async { Ok(case.shot::<_, C>(service, job, suite).await.unwrap()) })
//     }
// }
// impl<'a, S, C> Service<Testcase<C::ReqSource, C::ResSink>> for &'a SuiteService<'a, S, C>
// where
//     S: 'a + Clone + Service<C::Request, Response = C::Response> + Send,
//     C: Contract<S>,
//     C::Service: for<'x> Service<RequestSource<&'x C::ReqSource>> + Send,
//     for<'x> <C::Service as Service<RequestSource<&'x C::ReqSource>>>::Future: Send,
//     C::ReqSource: Send + Sync + 'static,
//     C::ResSink: Send + Sync + 'static,
// {
//     type Response = CaseReport<C::ReqSource, C::ResSink>;
//     type Error = S::Error;
//     type Future = Pin<Box<dyn 'a + Future<Output = Result<Self::Response, Self::Error>> + Send>>;

//     fn poll_ready(&mut self, _cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
//         Poll::Ready(Ok(()))
//     }

//     fn call(&mut self, case: Testcase<C::ReqSource, C::ResSink>) -> Self::Future {
//         let (service, job, suite) = (self.services.clone(), self.job, self.suite);
//         Box::pin(async { Ok(case.shot::<_, C>(service, job, suite).await.unwrap()) })
//     }
// }

#[derive(Debug, Clone, PartialEq)]
pub struct SuiteReport<Q, P> {
    cases: Vec<CaseReport<Q, P>>,
}
impl<Q, P> SuiteCase<Q, P> {
    pub async fn shot<M, S, N, C>(
        self,
        make_service: M,
        make_contract: &N,
        job: &JobSpec,
    ) -> crate::Result<SuiteReport<Q, P>>
    where
        M: Clone + MakeService<http::Uri, C::TransportReq, Service = S>,
        S: Clone + Service<C::TransportReq, Response = C::TransportRes> + Send,
        N: MakeContract<S, Q, C, Infallible>,
        C: Contract<S, ReqSource = Q, ResSink = P> + Layer<S>,
        C::Service: Service<C::Request, Response = C::Response> + Send,
        Q: RequestSource<C::Request>,
        P: ResponseSink<Result<C::Response, ServiceError<C, S>>>,
    {
        let buffers = if Hierarchy::Suite.contains(&job.sequential) { 1 } else { self.testcases.len().max(1) };
        let mut services = Destinations::default();
        for (d, http_newtype_serde::Uri(dest)) in self.suite.destinations.iter() {
            services.insert(
                d.to_string(),
                make_service.clone().make_service(dest.clone()).await.unwrap_or_else(|_| todo!()),
            );
        }
        let cases = futures::stream::iter(self.testcases)
            .map(|t| t.shot(&services, make_contract, job, &self.suite))
            .buffer_unordered(buffers)
            .try_collect()
            .await
            .unwrap_or_else(|_| todo!());
        Ok(SuiteReport { cases })
    }
}
