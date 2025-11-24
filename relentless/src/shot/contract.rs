use std::{
    fmt::{Debug, Display, Formatter},
    future::Future,
};

use tower::{Layer, Service};

use crate::shot::destinations::Destinations;

#[trait_variant::make(Send)]
pub trait Contract<T>: Sized {
    type ReqSource;
    type Request;
    type TransportReq;
    type TransportRes;
    type Response;
    type ResSink;

    type SignError;
    async fn new(service: T, req: &Self::ReqSource, res: &Self::ResSink) -> Result<Self, Self::SignError>;
}
pub type TransportError<T, C> = <T as Service<<C as Contract<T>>::TransportReq>>::Error;
pub type ServiceResponse<T, C> = <<C as Layer<T>>::Service as Service<<C as Contract<T>>::Request>>::Response;
pub type ServiceError<T, C> = <<C as Layer<T>>::Service as Service<<C as Contract<T>>::Request>>::Error;
pub type ReqSourceError<T, C> = <<C as Contract<T>>::ReqSource as RequestSource<<C as Contract<T>>::Request>>::Error;
pub type ResSinkError<T, C> =
    <<C as Contract<T>>::ResSink as ResponseSink<Result<ServiceResponse<T, C>, ServiceError<T, C>>>>::Error;

pub trait SignContract<T, Q, P, C, E>: AsyncFn(T, &Q, &P) -> Result<C, E> {
    fn sign_contract(&self, service: T, req: &Q, res: &P) -> impl Future<Output = Result<C, E>> {
        async { self(service, req, res).await }
    }
}
impl<F, S, Q, P, C, E> SignContract<S, Q, P, C, E> for F where F: AsyncFn(S, &Q, &P) -> Result<C, E> {}

#[trait_variant::make(Send)]
pub trait RequestSource<De> {
    type Error;
    async fn produce(&self, destination: &http::Uri, target: &str) -> Result<De, Self::Error>;
}

#[trait_variant::make(Send)]
pub trait ResponseSink<Se> {
    type Error;
    async fn consume(&self, res: Destinations<Se>) -> Result<(), Self::Error>;
}

pub type ShotError<T, C> = ErrorContract<
    <C as Contract<T>>::SignError,
    TransportError<T, C>,
    ServiceError<T, C>,
    ReqSourceError<T, C>,
    ResSinkError<T, C>,
>;
#[derive(Debug, Clone, PartialEq)]
pub enum ErrorContract<NE, TE, SE, QE, PE> {
    Sign(NE),
    Transport(TE),
    Service(SE),
    ReqSource(QE),
    ResSink(PE),
}
impl<NE, TE, SE, QE, PE> std::error::Error for ErrorContract<NE, TE, SE, QE, PE>
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
impl<NE, TE, SE, QE, PE> Display for ErrorContract<NE, TE, SE, QE, PE>
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
