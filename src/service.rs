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
