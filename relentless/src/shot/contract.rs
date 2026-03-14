use std::{
    fmt::{Debug, Display, Formatter},
    future::Future,
};

use semigroup::Semigroup;
use tower::{Layer, MakeService, Service};

use crate::{
    evaluator::evaluate::{Failure, Messages},
    shot::{destinations::Destinations, job::BasePath},
};

#[trait_variant::make(Send)]
pub trait Contract<T>: Sized {
    type Sign;
    type ReqSource;
    type Request;
    type TransportReq;
    type TransportRes;
    type Response;
    type ResSink;

    type SignError;
}
pub type MakeError<M, T, C> = <M as MakeService<http::Uri, <C as Contract<T>>::TransportReq>>::MakeError;
pub type TransportError<T, C> = <T as Service<<C as Contract<T>>::TransportReq>>::Error;
pub type ServiceResponse<T, C> = <<C as Layer<T>>::Service as Service<<C as Contract<T>>::Request>>::Response;
pub type ServiceError<T, C> = <<C as Layer<T>>::Service as Service<<C as Contract<T>>::Request>>::Error;
pub type ReqSourceError<T, C> = <<C as Contract<T>>::ReqSource as RequestSource<<C as Contract<T>>::Request>>::Error;
pub type ResSinkError<T, C> =
    <<C as Contract<T>>::ResSink as ResponseSink<Result<ServiceResponse<T, C>, ServiceError<T, C>>>>::Message;

pub trait SignContract<T, C> {
    type Error;
    fn sign_contract(
        &self,
        service: T,
        destination: &http::Uri,
        base_path: &Option<BasePath>,
    ) -> impl Future<Output = Result<C, Self::Error>>;
}

#[trait_variant::make(Send)]
pub trait RequestSource<De> {
    type Error;
    async fn produce(&self, destination: &http::Uri, target: &str) -> Result<De, Self::Error>;
}

#[trait_variant::make(Send)]
pub trait ResponseSink<Se> {
    type Message;
    async fn consume(&self, msg: &mut Messages<Self::Message>, res: Destinations<Se>) -> Result<(), Failure>;
}
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Default, Hash, Semigroup)]
#[semigroup(monoid, commutative, with = "semigroup::op::Sum")]
pub struct Evaluated {
    #[semigroup(with = "semigroup::op::All")]
    pub pass: bool,
    pub passed: usize,
    #[semigroup(with = "semigroup::op::All")]
    pub allow: bool,
    pub allowed: usize,
    pub times: usize,
}
impl Evaluated {
    pub fn new<T, E>(evaluated: &Result<T, E>, allow: Option<bool>) -> Self {
        let pass = evaluated.is_ok();
        let allow = pass || allow.unwrap_or_default();
        Self { pass, passed: pass as usize, allow, allowed: allow as usize, times: 1 }
    }
    pub fn assess(&self) -> Assessment {
        if self.pass {
            Assessment::Good
        } else if self.allow {
            Assessment::Acceptable
        } else {
            Assessment::Bad
        }
    }
}
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Assessment {
    Good,
    Acceptable,
    Poor,
    Bad,
}
impl Assessment {
    pub fn success(&self) -> bool {
        matches!(self, Self::Good | Self::Acceptable)
    }
    pub fn failure(&self) -> bool {
        matches!(self, Self::Bad | Self::Poor)
    }
}

pub type ContractError<T, C> = ContractErrorWrap<
    <C as Contract<T>>::SignError,
    TransportError<T, C>,
    ServiceError<T, C>,
    ReqSourceError<T, C>,
    ResSinkError<T, C>,
>;
#[derive(Debug, Clone, PartialEq)]
pub enum ContractErrorWrap<NE, TE, SE, QE, PE> {
    Sign(NE),
    Transport(TE),
    Service(SE),
    ReqSource(QE),
    ResSink(PE),
}
impl<NE, TE, SE, QE, PE> std::error::Error for ContractErrorWrap<NE, TE, SE, QE, PE>
where
    NE: std::error::Error + 'static,
    TE: std::error::Error + 'static,
    SE: std::error::Error + 'static,
    QE: std::error::Error + 'static,
    PE: std::error::Error + 'static,
{
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Sign(e) => Some(e),
            Self::Transport(e) => Some(e),
            Self::Service(e) => Some(e),
            Self::ReqSource(e) => Some(e),
            Self::ResSink(e) => Some(e),
        }
    }
}
impl<NE, TE, SE, QE, PE> Display for ContractErrorWrap<NE, TE, SE, QE, PE>
where
    NE: Display,
    TE: Display,
    SE: Display,
    QE: Display,
    PE: Display,
{
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Sign(e) => e.fmt(f),
            Self::Transport(e) => e.fmt(f),
            Self::Service(e) => e.fmt(f),
            Self::ReqSource(e) => e.fmt(f),
            Self::ResSink(e) => e.fmt(f),
        }
    }
}
