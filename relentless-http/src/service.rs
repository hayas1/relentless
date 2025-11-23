use std::{convert::Infallible, marker::PhantomData};

use relentless::shot::contract::Contract;
use tower::{layer::util::Identity, Layer, Service};

use crate::{request::HttpRequest, response::HttpResponse};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HttpContract<ReqB, ResB> {
    phantom: PhantomData<(ReqB, ResB)>,
}
impl<S, ReqB, ResB> Layer<S> for HttpContract<ReqB, ResB> {
    type Service = <Identity as Layer<S>>::Service;

    fn layer(&self, service: S) -> Self::Service {
        Identity::new().layer(service)
    }
}
impl<S, ReqB, ResB> Contract<S> for HttpContract<ReqB, ResB>
where
    S: Service<http::Request<ReqB>, Response = http::Response<ResB>> + Send,
    ReqB: Send,
    ResB: Send,
{
    type ReqSource = HttpRequest;
    type Request = http::Request<ReqB>;
    type TransportReq = http::Request<ReqB>;
    type TransportRes = http::Response<ResB>;
    type Response = http::Response<ResB>;
    type ResSink = HttpResponse;

    type ServiceError = S::Error;
    type Error = Infallible;

    async fn new(service: S, request: &Self::ReqSource) -> Result<Self, Self::Error> {
        Ok(HttpContract { phantom: PhantomData })
    }
}
