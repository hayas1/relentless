use std::{fmt::Debug, ops::Range, time::Duration};

use futures::{StreamExt, TryStreamExt};
use semigroup::Semigroup;
use serde::{Deserialize, Serialize};
use tower::{Layer, Service, ServiceExt};

use crate::{
    evaluator::evaluate::{MessageExt, Messages},
    shot::{
        contract::{Contract, ContractError, Evaluated, RequestSource, ResponseSink, ServiceError},
        destinations::Destinations,
    },
    template::{self, Template},
};

#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize, Semigroup)]
#[serde(deny_unknown_fields, rename_all = "kebab-case")]
pub struct Profile<Q, P> {
    #[serde(default)]
    pub request: Q,

    #[serde(default, with = "template::destinations_serde")]
    pub template: Destinations<Template>,
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
    #[allow(clippy::type_complexity)] // TODO
    #[tracing::instrument(name = "profile", skip(services))]
    pub async fn shot<T, C>(
        &self,
        services: &Destinations<C::Service>,
        destinations: &Destinations<http::Uri>,
        target: &str,
    ) -> Result<(Evaluated, Messages<P::Message>), ContractError<T, C>>
    where
        T: Service<C::TransportReq, Response = C::TransportRes>,
        C: Contract<T, ReqSource = Q, ResSink = P> + Layer<T>,
        C::Service: Clone + Service<C::Request, Response = C::Response>,
        Q: Debug + RequestSource<C::Request>,
        P: Debug + ResponseSink<Result<C::Response, ServiceError<T, C>>>,
    {
        let buffers = services.len().max(1);
        let responses = futures::stream::iter(services)
            .map(|(name, service)| {
                let template = self.template.get(name).cloned().unwrap_or_default();
                async move {
                    let destination = destinations.get(name).unwrap_or_else(|| todo!());
                    let request = self
                        .request
                        .produce(destination, target, &template)
                        .await
                        .map_err(ContractError::<T, C>::ReqSource)?;
                    let service = service.clone().oneshot(request);
                    let response = if let Some(timeout) = self.timeout {
                        match tokio::time::timeout(timeout, service).await {
                            Ok(response) => response,
                            Err(_) => Err(ContractError::<T, C>::Timeout(timeout))?,
                        }
                    } else {
                        service.await
                    };
                    Ok::<_, ContractError<T, C>>((name, response))
                }
            })
            .buffer_unordered(buffers)
            .try_collect()
            .await;
        let mut messages = Messages::new();
        match responses {
            Ok(responses) => {
                let evaluated = self.response.consume(&mut messages, responses).await;
                Ok((Evaluated::new(&evaluated, self.allow), messages))
            }
            e @ Err(ContractError::<T, C>::Timeout(t)) => {
                messages.error(<P as ResponseSink<Result<C::Response, ServiceError<T, C>>>>::Message::timeout(t));
                Ok((Evaluated::new(&e, self.allow), messages))
            }
            _ => todo!(),
        }
    }
}
