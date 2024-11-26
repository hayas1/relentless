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

pub mod origin_router {
    use std::{collections::HashMap, future::Future, marker::PhantomData, pin::Pin, task::Poll};

    use http::uri::Authority;
    use tower::Service;

    use crate::error::{AssaultError, Wrap};

    // TODO deref ?
    #[derive(Debug, Clone, PartialEq, Eq)]
    pub struct OriginRouter<S, B> {
        map: HashMap<Authority, S>,
        phantom: PhantomData<B>,
    }
    impl<S, B> OriginRouter<S, B> {
        pub fn new(map: HashMap<Authority, S>) -> Self {
            Self { map, phantom: PhantomData }
        }
    }
    impl<B, Req, S> Service<Req> for OriginRouter<S, B>
    where
        Req: From<http::Request<B>> + Into<http::Request<B>>,
        S: Service<Req>,
        S::Future: Send + 'static,
        Wrap: From<S::Error> + Send + 'static,
    {
        type Response = S::Response;
        type Error = Wrap;
        type Future = Pin<Box<dyn Future<Output = Result<Self::Response, Self::Error>>>>;

        fn poll_ready(&mut self, cx: &mut std::task::Context<'_>) -> Poll<Result<(), Self::Error>> {
            match self.map.values_mut().try_fold(true, |sum, s| Ok(sum && matches!(s.poll_ready(cx)?, Poll::Ready(()))))
            {
                Ok(true) => Poll::Ready(Ok(())),
                Ok(false) => Poll::Pending,
                Err(e) => Poll::Ready(Err(e)),
            }
        }

        fn call(&mut self, req: Req) -> Self::Future {
            let request: http::Request<B> = req.into();
            if let Some(s) = request.uri().authority().and_then(|a| self.map.get_mut(a)) {
                let fut = s.call(request.into());
                Box::pin(async { Ok(fut.await?) })
            } else {
                Box::pin(async { Err(AssaultError::CannotSpecifyService)? })
            }
        }
    }

    #[cfg(test)]
    mod tests {

        use http_body_util::{BodyExt, Collected, Empty};
        use relentless_dev_server::route::{self, counter::CounterResponse};

        use super::*;

        #[tokio::test]
        async fn test_origin_router() {
            let (service1, service2) = (route::app_with(Default::default()), route::app_with(Default::default()));
            let mut service = OriginRouter::new(
                [
                    (Authority::from_static("localhost:3000"), service1),
                    (Authority::from_static("localhost:3001"), service2),
                ]
                .into_iter()
                .collect(),
            );
            let request1 =
                http::Request::builder().uri("http://localhost:3000/counter/increment").body(Empty::new()).unwrap();
            let response1 = service.call(request1).await.unwrap();
            assert_eq!(response1.status(), 200);
            let bytes1 = BodyExt::collect(response1.into_body()).await.map(Collected::to_bytes).unwrap();
            let count1: CounterResponse<i64> = serde_json::from_slice(&bytes1).unwrap();
            assert_eq!(count1, CounterResponse { count: 1 });

            let request2 = http::Request::builder().uri("http://localhost:3001/counter").body(Empty::new()).unwrap();
            let response2 = service.call(request2).await.unwrap();
            assert_eq!(response2.status(), 200);
            let bytes2 = BodyExt::collect(response2.into_body()).await.map(Collected::to_bytes).unwrap();
            let count2: CounterResponse<i64> = serde_json::from_slice(&bytes2).unwrap();
            assert_eq!(count2, CounterResponse { count: 0 });
        }

        #[tokio::test]
        async fn test_origin_router_not_found() {
            let mut service = OriginRouter::new(
                [(Authority::from_static("localhost:3000"), route::app(Default::default()))].into_iter().collect(),
            );
            let request = http::Request::builder().uri("http://localhost:3000").body(Empty::new()).unwrap();
            let response = service.call(request).await.unwrap();
            assert_eq!(response.status(), 200);
            assert_eq!(
                &BodyExt::collect(response.into_body()).await.map(Collected::to_bytes).unwrap()[..],
                b"Hello World"
            );

            let request = http::Request::builder().uri("http://localhost:8000").body(Empty::new()).unwrap();
            let err = service.call(request).await.unwrap_err();
            assert!(matches!(err.downcast_ref(), Some(AssaultError::CannotSpecifyService)));
        }
    }
}

#[derive(Debug)]
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
impl From<Bytes> for BytesBody {
    fn from(val: Bytes) -> Self {
        if val.is_empty() {
            BytesBody(http_body_util::Empty::new().map_err(Wrap::error).boxed())
        } else {
            BytesBody(http_body_util::Full::new(val).map_err(Wrap::error).boxed())
        }
    }
}
impl FromBodyStructure for BytesBody {
    fn from_body_structure(val: BodyStructure) -> Self {
        match val {
            BodyStructure::Empty => Bytes::new().into(),
            BodyStructure::PlainText(s) => Bytes::from(s).into(),
            #[cfg(feature = "json")]
            BodyStructure::Json(body) => Bytes::from(serde_json::to_vec(&body).unwrap()).into(),
        }
    }
}

pub trait FromBodyStructure {
    fn from_body_structure(structure: BodyStructure) -> Self;
}
impl<T> FromBodyStructure for T
where
    T: Body + From<Bytes> + Default,
{
    fn from_body_structure(structure: BodyStructure) -> Self {
        match structure {
            BodyStructure::Empty => Default::default(),
            BodyStructure::PlainText(s) => Bytes::from(s).into(),
            #[cfg(feature = "json")]
            BodyStructure::Json(_) => Bytes::from(serde_json::to_vec(&structure).unwrap()).into(),
        }
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
}
