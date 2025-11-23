use std::future::Future;

use tower::{Layer, Service};

use crate::shot::destinations::Destinations;

#[trait_variant::make(Send)]
pub trait Contract<S>: Sized {
    type ReqSource;
    type Request;
    type TransportReq;
    type TransportRes;
    type Response;
    type ResSink;

    type SignError;
    async fn new(service: S, req: &Self::ReqSource, res: &Self::ResSink) -> Result<Self, Self::SignError>;
}
pub type TransportError<C, S> = <S as Service<<C as Contract<S>>::TransportReq>>::Error;
pub type ServiceError<C, S> = <<C as Layer<S>>::Service as Service<<C as Contract<S>>::Request>>::Error;

pub trait SignContract<S, Q, P, C, E>: AsyncFn(S, &Q, &P) -> Result<C, E> {
    fn sign_contract(&self, service: S, req: &Q, res: &P) -> impl Future<Output = Result<C, E>> {
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
