use std::future::Future;

use tower::{Layer, Service};

use crate::shot::destinations::Destinations;

#[trait_variant::make(Send)]
pub trait Contract<S> {
    type ReqSource;
    type Request;
    type TransportReq;
    type TransportRes;
    type Response;
    type ResSink;
}
pub type TransportError<C, S> = <S as Service<<C as Contract<S>>::TransportReq>>::Error;
pub type ServiceError<C, S> = <<C as Layer<S>>::Service as Service<<C as Contract<S>>::Request>>::Error;

pub trait MakeContract<S, Q, C, E>: AsyncFn(S, &Q) -> Result<C, E> {
    fn make_contract(&self, service: S, request: &Q) -> impl Future<Output = Result<C, E>> {
        async { self(service, request).await }
    }
}
impl<F, S, Q, C, E> MakeContract<S, Q, C, E> for F where F: AsyncFn(S, &Q) -> Result<C, E> {}

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
