use std::{
    future::Future,
    marker::PhantomData,
    pin::Pin,
    task::{Context, Poll},
};

use bytes::Bytes;
use http_body::Body;
use http_body_util::{combinators::BoxBody, BodyExt};
use tower::Service;

use crate::{
    config::BodyStructure,
    error::{Wrap, WrappedResult},
};

#[cfg(feature = "default-http-client")]
pub const APP_USER_AGENT: &str = concat!(env!("CARGO_PKG_NAME"), "/", env!("CARGO_PKG_VERSION"),);

#[derive(Debug)]
#[cfg(feature = "default-http-client")]
pub struct DefaultHttpClient<ReqB, ResB> {
    client: reqwest::Client,
    phantom: PhantomData<(ReqB, ResB)>,
}
#[cfg(feature = "default-http-client")]
impl<ReqB, ResB> DefaultHttpClient<ReqB, ResB> {
    pub async fn new() -> WrappedResult<Self> {
        // TODO use hyper ? continue to use reqwest's rich client?
        let client = reqwest::Client::builder().user_agent(APP_USER_AGENT).build()?;
        Ok(Self { client, phantom: PhantomData })
    }
}

#[cfg(feature = "default-http-client")]
impl<ReqB, ResB> Service<http::Request<ReqB>> for DefaultHttpClient<ReqB, ResB>
where
    ReqB: Into<reqwest::Body>,
    ResB: From<reqwest::Body>,
{
    type Response = http::Response<ResB>;
    type Error = reqwest::Error;
    type Future = Pin<Box<dyn Future<Output = Result<Self::Response, Self::Error>>>>;

    fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        self.client.poll_ready(cx)
    }

    fn call(&mut self, req: http::Request<ReqB>) -> Self::Future {
        let req = req.try_into().unwrap(); // TODO handle error
        let fut = self.client.call(req);
        Box::pin(async {
            fut.await.map(|res| {
                let b = http::Response::<reqwest::Body>::from(res);
                let (parts, incoming) = b.into_parts();
                http::Response::from_parts(parts, incoming.into())
            })
        })
    }
}

pub struct BytesBody(BoxBody<Bytes, crate::Error>);
impl Body for BytesBody {
    type Data = Bytes;
    type Error = crate::Error;

    fn poll_frame(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
    ) -> Poll<Option<Result<http_body::Frame<Self::Data>, Self::Error>>> {
        Pin::new(&mut self.0).poll_frame(cx)
    }
    fn is_end_stream(&self) -> bool {
        self.0.is_end_stream()
    }
    fn size_hint(&self) -> http_body::SizeHint {
        self.0.size_hint()
    }
}
impl FromBodyStructure for BytesBody {
    fn from_body_structure(val: BodyStructure) -> Self {
        match val {
            BodyStructure::Empty => BytesBody(http_body_util::Empty::new().map_err(Wrap::error).boxed()),
        }
    }
}

pub trait FromBodyStructure {
    fn from_body_structure(val: BodyStructure) -> Self;
}
impl<T> FromBodyStructure for T
where
    T: Body + Default, // TODO other than Default
{
    fn from_body_structure(body: BodyStructure) -> Self {
        match body {
            BodyStructure::Empty => Default::default(),
        }
    }
}

pub trait IntoBytesBody {
    fn into_bytes_body(self) -> BytesBody;
}
impl<T> IntoBytesBody for T
where
    T: Body<Data = Bytes> + Send + Sync + 'static,
    T::Error: std::error::Error + Send + Sync,
{
    fn into_bytes_body(self) -> BytesBody {
        BytesBody(self.map_err(Wrap::error).boxed())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    #[cfg(feature = "default-http-client")]
    async fn test_default_http_client() {
        use tower::ServiceExt;

        let server = httptest::Server::run();
        server.expect(
            httptest::Expectation::matching(httptest::matchers::request::method_path("GET", "/"))
                .respond_with(httptest::responders::status_code(200).body("hello world")),
        );

        let mut client = DefaultHttpClient::<Bytes, reqwest::Body>::new().await.unwrap();
        let request = http::Request::builder().uri(server.url("/")).body(Bytes::new()).unwrap();
        let res: reqwest::Response = client.ready().await.unwrap().call(request).await.unwrap().into();
        assert_eq!(res.status(), 200);
        assert_eq!(res.text().await.unwrap(), "hello world");
    }

    #[tokio::test]
    async fn test_from_body_structure_empty() {
        let bytes_body = BytesBody::from_body_structure(BodyStructure::Empty);
        assert!(bytes_body.is_end_stream());

        let bytes1 = BodyExt::collect(http_body_util::Empty::<Bytes>::from_body_structure(BodyStructure::Empty))
            .await
            .map(http_body_util::Collected::to_bytes)
            .unwrap();
        let bytes2 = BodyExt::collect(http_body_util::Empty::<Bytes>::new())
            .await
            .map(http_body_util::Collected::to_bytes)
            .unwrap();
        assert_eq!(bytes1, bytes2);
    }
}
