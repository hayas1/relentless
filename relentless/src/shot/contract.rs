use tower::Layer;

use crate::shot::destinations::Destinations;

#[trait_variant::make(Send)]
pub trait Contract<S>: Sized + Layer<S> {
    type ReqSource;
    type Request;
    type TransportReq;
    type ResSink;
    type Response;
    type TransportRes;
    type ServiceError;
    type Error;

    async fn new(service: S, request: &Self::ReqSource) -> Result<Self, Self::Error>;
}

#[trait_variant::make(Send)]
pub trait RequestSource<De> {
    type Error;
    async fn produce(&self, destination: &http::Uri, target: &str) -> Result<De, Self::Error>;
}

#[trait_variant::make(Send)]
pub trait ResponseSink<Se> {
    async fn consume(&self, res: Destinations<Se>) -> bool;
}
