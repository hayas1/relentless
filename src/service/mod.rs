pub mod evaluate;
pub mod record;

use std::{
    future::Future,
    marker::PhantomData,
    pin::Pin,
    task::{Context, Poll},
};

use bytes::Bytes;
use http::HeaderMap;
use http_body::Body;
use tower::Service;

use crate::{
    error::{Wrap, WrappedResult},
    interface::config::{HttpBody, HttpRequest},
    template::Template,
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
impl<ReqB, ResB> Clone for DefaultHttpClient<ReqB, ResB> {
    fn clone(&self) -> Self {
        // derive(Clone) do not implement Clone when ReqB or ResB are not implement Clone
        // https://github.com/rust-lang/rust/issues/26925
        Self { client: self.client.clone(), phantom: PhantomData }
    }
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
        type Error = crate::Error;
        type Future = Pin<Box<dyn Future<Output = Result<Self::Response, Self::Error>> + Send>>;

        fn poll_ready(&mut self, cx: &mut std::task::Context<'_>) -> Poll<Result<(), Self::Error>> {
            match self.map.values_mut().try_fold(true, |sum, s| {
                Ok(sum && matches!(s.poll_ready(cx).map_err(crate::Error::wrap)?, Poll::Ready(())))
            }) {
                Ok(true) => Poll::Ready(Ok(())),
                Ok(false) => Poll::Pending,
                Err(e) => Poll::Ready(Err(e)),
            }
        }

        fn call(&mut self, req: Req) -> Self::Future {
            let request: http::Request<B> = req.into();
            if let Some(s) = request.uri().authority().and_then(|a| self.map.get_mut(a)) {
                let fut = s.call(request.into());
                Box::pin(async { fut.await.map_err(crate::Error::wrap) })
            } else {
                Box::pin(async { Err(Wrap::error(AssaultError::CannotSpecifyService)) })
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
pub trait RequestFactory<R> {
    type Error;
    fn produce(&self, destination: &http::Uri, target: &str, template: &Template) -> Result<R, Self::Error>;
}
impl<B> RequestFactory<http::Request<B>> for HttpRequest
where
    B: Body,
    HttpBody: BodyFactory<B>,
    Wrap: From<<HttpBody as BodyFactory<B>>::Error>,
{
    type Error = Wrap;
    fn produce(
        &self,
        destination: &http::Uri,
        target: &str,
        template: &Template,
    ) -> Result<http::Request<B>, Self::Error> {
        let HttpRequest { no_additional_headers, method, headers, body } = self;
        let uri = http::uri::Builder::from(destination.clone()).path_and_query(template.render(target)?).build()?;
        let unwrapped_method = method.as_ref().map(|m| (**m).clone()).unwrap_or_default();
        let unwrapped_headers: HeaderMap = headers.as_ref().map(|h| (**h).clone()).unwrap_or_default();
        // .into_iter().map(|(k, v)| (k, template.render_as_string(v))).collect(); // TODO template with header
        let (actual_body, additional_headers) = body.clone().unwrap_or_default().body_with_headers(template)?;

        let mut request = http::Request::builder().uri(uri).method(unwrapped_method).body(actual_body)?;
        let header_map = request.headers_mut();
        header_map.extend(unwrapped_headers);
        if !no_additional_headers {
            header_map.extend(additional_headers);
        }
        Ok(request)
    }
}

pub trait BodyFactory<B: Body> {
    type Error;
    fn produce(&self, template: &Template) -> Result<B, Self::Error>;
}
impl<B> BodyFactory<B> for HttpBody
where
    B: Body + From<Bytes> + Default,
{
    type Error = Wrap;
    fn produce(&self, template: &Template) -> Result<B, Self::Error> {
        match self {
            HttpBody::Empty => Ok(Default::default()),
            HttpBody::Plaintext(s) => Ok(Bytes::from(template.render(s).unwrap_or(s.to_string())).into()),
            #[cfg(feature = "json")]
            HttpBody::Json(_) => Ok(Bytes::from(serde_json::to_vec(&self)?).into()),
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
