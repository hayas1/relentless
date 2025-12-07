use std::{convert::Infallible, marker::PhantomData};

use relentless::shot::contract::{Contract, SignContract};
use serde::{Deserialize, Serialize};
use tower::{layer::util::Identity, Layer, Service};

use crate::{request::HttpRequest, response::HttpResponse};

#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct HttpContract<ReqB, ResB> {
    phantom: PhantomData<(ReqB, ResB)>,
}
impl<T, ReqB, ResB> SignContract<T, Self> for HttpContract<ReqB, ResB> {
    type Error = Infallible;
    async fn sign_contract(&self, _: T, _: &http::Uri) -> Result<Self, Self::Error> {
        Ok(HttpContract { phantom: PhantomData })
    }
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
    type Sign = Self;
    type ReqSource = HttpRequest;
    type Request = Self::TransportReq;
    type TransportReq = http::Request<ReqB>;
    type TransportRes = http::Response<ResB>;
    type Response = Self::TransportRes;
    type ResSink = HttpResponse;

    type SignError = Infallible;
}
