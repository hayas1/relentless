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

pub struct RequestSource<'a, Q> {
    pub destination: &'a http::Uri,
    pub target: &'a str,
    pub source: &'a Q,
    // pub template: Template
}

#[trait_variant::make(Send)]
pub trait ResponseSink<S> {
    async fn consume(&self, res: Destinations<S>) -> bool;
}
