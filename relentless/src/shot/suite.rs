use futures::{StreamExt, TryStreamExt};
use http::Uri;
use semigroup::{CombineIterator, Lazy, Semigroup};
use serde::{Deserialize, Serialize};
use tower::{Layer, MakeService, Service};

use crate::{
    http_newtype_serde,
    shot::{
        contract::{Contract, RequestSource, ResponseSink, ServiceError, SignContract},
        destinations::Destinations,
        hierarchy::Hierarchy,
        job::JobSpec,
        profile::Profile,
        testcase::{Aggregate, CaseReport, Testcase},
    },
};

#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "kebab-case")]
pub struct SuiteCase<C, Q, P> {
    #[serde(flatten)]
    pub suite: Suite<C, Q, P>,

    #[serde(default)]
    pub testcases: Vec<Testcase<Q, P>>,
}

#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "kebab-case")]
pub struct Suite<C, Q, P> {
    pub name: String,
    pub destinations: Destinations<http_newtype_serde::Uri>,
    #[serde(default)]
    pub profile: Profile<Q, P>,
    pub contract: Option<C>,
}
// impl<Q, P> Suite<Q, P> {
//     pub async fn transport<'a, M, S, C>(
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
//         let (transport, job, suite) = (self.services.clone(), self.job, self.suite);
//         Box::pin(async { Ok(case.shot::<_, C>(transport, job, suite).await.unwrap()) })
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
//         let (transport, job, suite) = (self.services.clone(), self.job, self.suite);
//         Box::pin(async { Ok(case.shot::<_, C>(transport, job, suite).await.unwrap()) })
//     }
// }

#[derive(Debug, Clone, PartialEq)]
pub struct SuiteReport<'a, C, Q, P> {
    pub destinations: Lazy<Destinations<Uri>>,
    pub suite: &'a Suite<C, Q, P>,
    pub cases: Vec<CaseReport<'a, Q, P>>,
    pub aggregate: Aggregate,
}
impl<S, Q, P> SuiteCase<S, Q, P> {
    pub async fn shot<M, T, C>(&self, make_service: M, job: &JobSpec) -> crate::Result<SuiteReport<S, Q, P>>
    where
        M: Clone + MakeService<http::Uri, C::TransportReq, Service = T>,
        T: Clone + Service<C::TransportReq, Response = C::TransportRes> + Send,
        S: SignContract<T, C> + Default,
        C: Contract<T, Sign = S, ReqSource = Q, ResSink = P> + Layer<T>,
        C::Service: Clone + Service<C::Request, Response = C::Response> + Send,
        Q: Clone + Semigroup + RequestSource<C::Request>,
        P: Clone + Semigroup + ResponseSink<Result<C::Response, ServiceError<T, C>>>,
    {
        let buffers = if Hierarchy::Suite.contains(&job.sequential) { 1 } else { self.testcases.len().max(1) };
        let destinations = job.destinations(&self.suite.destinations).unwrap_or_else(|e| todo!("{e}"));
        let mut services = Destinations::default();
        for (d, dest) in destinations.combine_rev_clone().iter() {
            let transport = make_service.clone().make_service(dest.clone()).await.unwrap_or_else(|_| todo!());
            let contract = self
                .suite
                .contract
                .as_ref()
                .unwrap_or(&Default::default())
                .sign_contract(transport.clone(), dest)
                .await
                .unwrap_or_else(|_| todo!());
            services.insert(d.to_string(), contract.layer(transport));
        }
        let cases: Vec<_> = futures::stream::iter(&self.testcases)
            .map(|t| t.shot(&services, job, &self.suite))
            .buffer_unordered(buffers)
            .try_collect()
            .await?;
        let aggregate = cases.iter().map(|c| c.aggregate.clone()).combine();
        Ok(SuiteReport { destinations, suite: &self.suite, cases, aggregate })
    }
}
