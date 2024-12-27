use std::{collections::HashMap, future::Future, marker::PhantomData, pin::Pin, task::Poll};

use http::uri::Authority;
use tower::Service;

use crate::error2::{AssaultError, RelentlessError};

#[derive(Debug, PartialEq, Eq, Default)]
pub struct OriginRouter<S, B> {
    map: HashMap<Authority, S>,
    phantom: PhantomData<B>,
}
impl<S, B> Clone for OriginRouter<S, B>
where
    S: Clone,
{
    fn clone(&self) -> Self {
        // derive(Clone) do not implement Clone when ReqB or ResB are not implement Clone
        // https://github.com/rust-lang/rust/issues/26925
        Self::new(self.map.clone())
    }
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
    S::Error: std::error::Error + Send + Sync + 'static,
{
    type Response = S::Response;
    type Error = RelentlessError;
    type Future = Pin<Box<dyn Future<Output = Result<Self::Response, Self::Error>> + Send>>;

    fn poll_ready(&mut self, cx: &mut std::task::Context<'_>) -> Poll<Result<(), Self::Error>> {
        match self.map.values_mut().try_fold(true, |sum, s| {
            Ok(sum && matches!(s.poll_ready(cx).map_err(RelentlessError::boxed)?, Poll::Ready(())))
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
            Box::pin(async { fut.await.map_err(RelentlessError::boxed) })
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
        assert_eq!(&BodyExt::collect(response.into_body()).await.map(Collected::to_bytes).unwrap()[..], b"Hello World");

        let request = http::Request::builder().uri("http://localhost:8000").body(Empty::new()).unwrap();
        let err = service.call(request).await.unwrap_err();
        assert!(matches!(err.downcast_ref(), Some(AssaultError::CannotSpecifyService)));
    }
}
