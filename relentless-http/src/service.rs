use std::{
    convert::Infallible,
    future::Future,
    marker::PhantomData,
    pin::Pin,
    task::{Context, Poll},
};

use relentless::shot::contract::{Contract, RequestSource, ResponseSink};
use serde::{Deserialize, Serialize};
use tower::{layer::util::Identity, Layer, Service};

use crate::{request::HttpRequest, response::HttpResponse};

#[derive(Debug, Default)]
pub struct ReqwestClient<ReqB, ResB> {
    client: reqwest::Client,
    phantom: PhantomData<(ReqB, ResB)>,
}
impl<ReqB, ResB> Clone for ReqwestClient<ReqB, ResB> {
    fn clone(&self) -> Self {
        // derive(Clone) do not implement Clone when ReqB or ResB are not implement Clone
        // https://github.com/rust-lang/rust/issues/26925
        Self { client: self.client.clone(), phantom: PhantomData }
    }
}
impl<ReqB, ResB> ReqwestClient<ReqB, ResB> {
    pub fn user_agent() -> &'static str {
        concat!(env!("CARGO_PKG_NAME"), "/", env!("CARGO_PKG_VERSION"))
    }
    pub async fn new() -> relentless::Result<Self> {
        let client =
            reqwest::Client::builder().user_agent(Self::user_agent()).build().map_err(relentless::Error::boxed)?;
        Ok(Self { client, phantom: PhantomData })
    }
}
impl<ReqB, ResB> Service<http::Request<ReqB>> for ReqwestClient<ReqB, ResB>
where
    ReqB: Into<reqwest::Body>,
    ResB: From<reqwest::Body>,
{
    type Response = http::Response<ResB>;
    type Error = reqwest::Error;
    type Future = Pin<Box<dyn Future<Output = Result<Self::Response, Self::Error>> + Send>>;

    fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        self.client.poll_ready(cx)
    }

    fn call(&mut self, request: http::Request<ReqB>) -> Self::Future {
        match request.try_into() {
            Ok(req) => {
                let fut = self.client.call(req);
                Box::pin(async {
                    fut.await.map(|res| {
                        let b = http::Response::<reqwest::Body>::from(res);
                        let (parts, incoming) = b.into_parts();
                        http::Response::from_parts(parts, incoming.into())
                    })
                })
            }
            Err(e) => Box::pin(async { Err(e) }),
        }
    }
}

#[cfg(test)]
mod tests {
    use bytes::Bytes;
    use tower::ServiceExt;

    use super::*;

    #[tokio::test]
    async fn test_default_http_client() {
        let server = httptest::Server::run();
        server.expect(
            httptest::Expectation::matching(httptest::matchers::request::method_path("GET", "/"))
                .respond_with(httptest::responders::status_code(200).body("hello world")),
        );

        let client = ReqwestClient::<Bytes, reqwest::Body>::new().await.unwrap();
        let request = http::Request::builder().uri(server.url("/")).body(Bytes::new()).unwrap();
        let res: reqwest::Response = client.oneshot(request).await.unwrap().into();
        assert_eq!(res.status(), 200);
        assert_eq!(res.text().await.unwrap(), "hello world");
    }
}
