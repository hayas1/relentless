use std::{ops::Range, time::Duration};

use futures::{StreamExt, TryStreamExt};
use semigroup::Semigroup;
use serde::{Deserialize, Serialize};
use tower::{Layer, Service, ServiceExt};

use crate::shot::{
    contract::{Contract, ContractError, RequestSource, ResponseSink, ServiceError},
    destinations::Destinations,
};

#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize, Semigroup)]
#[serde(deny_unknown_fields, rename_all = "kebab-case")]
pub struct Profile<Q, P> {
    #[serde(default)]
    pub request: Q,

    // #[serde(default, with = "transpose::transpose_template_serde")]
    // pub template: Destinations<Template>,
    #[serde(default)]
    pub repeat: Repeat,
    #[serde(default)]
    #[semigroup(with = "semigroup::op::Coalesce")]
    pub timeout: Option<Duration>, // TODO parse from string? https://crates.io/crates/humantime ?
    #[serde(default)]
    #[semigroup(with = "semigroup::op::Coalesce")]
    pub allow: Option<bool>,

    #[serde(default)]
    pub response: P,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Default, Hash, Serialize, Deserialize, Semigroup)]
#[serde(deny_unknown_fields, rename_all = "kebab-case")]
#[semigroup(with = "semigroup::op::Coalesce")]
pub struct Repeat(pub Option<usize>);
impl Repeat {
    pub fn range(&self) -> Range<usize> {
        0..self.times()
    }
    pub fn times(&self) -> usize {
        self.0.unwrap_or(1)
    }
}

impl<Q, P> Profile<Q, P> {
    pub async fn shot<T, C>(
        &self,
        services: &Destinations<C::Service>,
        destinations: &Destinations<http::Uri>,
        target: &str,
    ) -> Result<(), ContractError<T, C>>
    where
        T: Service<C::TransportReq, Response = C::TransportRes>,
        C: Contract<T, ReqSource = Q, ResSink = P> + Layer<T>,
        C::Service: Clone + Service<C::Request, Response = C::Response>,
        Q: RequestSource<C::Request>,
        P: ResponseSink<Result<C::Response, ServiceError<T, C>>>,
    {
        let buffers = services.len().max(1);
        let responses = futures::stream::iter(services)
            .map(|(name, service)| async move {
                let destination = destinations.get(name).unwrap_or_else(|| todo!());
                let request =
                    self.request.produce(destination, target).await.map_err(ContractError::<T, C>::ReqSource)?;
                let response = service.clone().oneshot(request).await;
                Ok((name, response))
            })
            .buffer_unordered(buffers)
            .try_collect()
            .await?;
        self.response.consume(responses).await.map_err(ContractError::<T, C>::ResSink)
    }
}

// pub struct ProfileService<T, C>
// where
//     C: Contract<T> + Layer<T>,
// {
//     services: Destinations<C::Service>,
//     profile: Arc<Profile<C::ReqSource, C::ResSink>>,
// }
// impl<T, C> Service<(&Destinations<http::Uri>, &str)> for ProfileService<T, C>
// where
//     T: Clone + Service<C::TransportReq, Response = C::TransportRes> + Send,
//     C: Contract<T> + Layer<T>,
//     C::Service: Clone + Service<C::Request, Response = C::Response> + Send + 'static,
//     <C::Service as Service<C::Request>>::Future: Send,
//     <C::Service as Service<C::Request>>::Error: Send,
//     C::Request: Send,
//     C::Response: Send,
//     C::ReqSource: RequestSource<C::Request> + Sync + 'static,
//     C::ResSink: ResponseSink<Result<C::Response, ServiceError<T, C>>> + Sync + 'static,
// {
//     type Response = ();
//     type Error = ContractError<T, C>;
//     type Future = Pin<Box<dyn Future<Output = Result<Self::Response, Self::Error>> + Send>>;

//     fn poll_ready(&mut self, _cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
//         Poll::Ready(Ok(()))
//     }

//     fn call(&mut self, (destinations, target): (&Destinations<http::Uri>, &str)) -> Self::Future {
//         let buffers = self.services.len().max(1);
//         let (services, profile) = (self.services.clone(), self.profile.clone());
//         let (destinations, target) = (destinations.clone(), target.to_string());
//         Box::pin(async move {
//             let (profile_ref, destinations_ref, target_ref) = (&profile, &destinations, &target);
//             let responses = futures::stream::iter(services)
//                 .map(move |(name, service)| {
//                     let (profile, destinations, target) = (profile_ref, destinations_ref, target_ref);
//                     async move {
//                         let dst = destinations.get(&name).unwrap_or_else(|| todo!());
//                         let request =
//                             profile.request.produce(dst, target).await.map_err(ContractError::<T, C>::ReqSource)?;
//                         let response = service.oneshot(request).await;
//                         Ok((name, response))
//                     }
//                 })
//                 .buffer_unordered(buffers)
//                 .try_collect()
//                 .await
//                 .unwrap_or_else(|_: ContractError<T, C>| todo!());
//             profile.response.consume(responses).await.map_err(ContractError::<T, C>::ResSink)
//         })
//     }
// }
