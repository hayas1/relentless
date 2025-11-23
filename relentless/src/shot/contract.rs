use std::future::Future;

use crate::shot::destinations::Destinations;

#[trait_variant::make(Send)]
pub trait Contract<S>: Sized {
    type ReqSource;
    type Request;
    type TransportReq;
    type TransportRes;
    type Response;
    type ResSink;

    type ServiceError;
}

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
