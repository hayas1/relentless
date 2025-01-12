// ##### TODO record to sqlite or duckdb with measure.rs #####
// **DEPRECATED** record request / response is experimental feature

use std::{
    fs::File,
    future::Future,
    io::Write,
    path::PathBuf,
    pin::Pin,
    task::{Context, Poll},
};

use bytes::Bytes;
use http::{header::CONTENT_TYPE, Method};
// use http_body::Body;
// use http_body_util::{BodyExt, Collected};
use serde::{de::DeserializeOwned, Deserialize};
use tower::{Layer, Service};

use crate::error::IntoResult;
// #[cfg(feature = "json")]
// use crate::implement::service_grpc::client::DefaultGrpcRequest;

pub trait Recordable: Sized + Send {
    type Error;
    fn record_raw<W: std::io::Write + Send>(
        self,
        w: &mut W,
    ) -> impl std::future::Future<Output = Result<(), Self::Error>> + Send;
    fn extension(&self) -> &'static str {
        "txt"
    }
    fn record<W: std::io::Write + Send>(self, w: &mut W) -> impl Future<Output = Result<(), Self::Error>> + Send {
        async { self.record_raw(w).await }
    }
}
pub trait CloneCollected: Sized {
    type CollectError;
    /// once consume body to record, and reconstruct to request/response
    fn clone_collected(self) -> impl Future<Output = Result<(Self, Self), Self::CollectError>> + Send;
}
pub trait RecordableRequest {
    fn record_dir(&self) -> PathBuf;
}

#[derive(Debug, Clone, PartialEq, Eq, Default, Hash)]
pub struct RecordLayer {
    path: Option<PathBuf>,
}
impl RecordLayer {
    pub fn new(path: Option<PathBuf>) -> Self {
        Self { path }
    }
}
impl<S> Layer<S> for RecordLayer {
    type Service = RecordService<S>;
    fn layer(&self, inner: S) -> Self::Service {
        let path = self.path.clone();
        RecordService { path, inner }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Default, Hash)]
pub struct RecordService<S> {
    path: Option<PathBuf>,
    inner: S,
}
impl<S, Req, Res> Service<Req> for RecordService<S>
where
    Req: Recordable + CloneCollected + RecordableRequest + Send + 'static,
    Req::Error: std::error::Error + Send + Sync + 'static,
    Req::CollectError: std::error::Error + Send + Sync + 'static,
    Res: Recordable + CloneCollected + Send,
    Res::Error: std::error::Error + Send + Sync + 'static,
    Res::CollectError: std::error::Error + Send + Sync + 'static,
    S: Service<Req, Response = Res> + Clone + Send + 'static,
    S::Future: Send + 'static,
    S::Error: std::error::Error + Send + Sync + 'static,
{
    type Response = S::Response;
    type Error = crate::Error;
    type Future = Pin<Box<dyn Future<Output = Result<Self::Response, Self::Error>> + Send>>;

    fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        self.inner.poll_ready(cx).box_err()
    }

    fn call(&mut self, request: Req) -> Self::Future {
        let paths = (|p: Option<&PathBuf>| {
            // TODO path will be uri ... (if implement template, it will not be in path)
            // TODO timestamp or repeated number
            // TODO join path (absolute) https://github.com/rust-lang/rust/issues/16507
            let dir = p?.join(request.record_dir());
            std::fs::create_dir_all(&dir).ok()?;
            writeln!(File::create(p?.join(".gitignore")).ok()?, "*").ok()?; // TODO hardcode...
            Some(((dir.join("raw_request"), dir.join("request")), (dir.join("raw_response"), dir.join("response"))))
        })(self.path.as_ref());

        if let Some(((path_raw_req, path_req), (path_raw_res, path_res))) = paths {
            let mut cloned_inner = self.inner.clone();
            Box::pin(async move {
                let (request, recordable_raw_req) = request.clone_collected().await.box_err()?;
                recordable_raw_req
                    .record_raw(&mut File::create(path_raw_req.with_extension("txt")).box_err()?)
                    .await
                    .box_err()?;
                let (request, recordable_req) = request.clone_collected().await.box_err()?;
                let req_record_extension = recordable_req.extension();
                recordable_req
                    .record(&mut File::create(path_req.with_extension(req_record_extension)).box_err()?)
                    .await
                    .box_err()?;

                let response = cloned_inner.call(request).await.box_err()?;

                let (response, recordable_raw_res) = response.clone_collected().await.box_err()?;
                recordable_raw_res
                    .record_raw(&mut File::create(path_raw_res.with_extension("txt")).box_err()?)
                    .await
                    .box_err()?;
                let (response, recordable_res) = response.clone_collected().await.box_err()?;
                let res_record_extension = recordable_res.extension();
                recordable_res
                    .record(&mut File::create(path_res.with_extension(res_record_extension)).box_err()?)
                    .await
                    .box_err()?;

                Ok(response)
            })
        } else {
            let fut = self.inner.call(request);
            Box::pin(async move { fut.await.box_err() })
        }
    }
}

#[cfg(test)]
mod tests {
    // use bytes::Bytes;
    // use http::Method;

    // use super::*;

    // #[tokio::test]
    // async fn test_empty_body_request() {
    //     let request = http::Request::builder()
    //         .method(Method::GET)
    //         .uri("http://localhost:3000")
    //         .body(http_body_util::Empty::<Bytes>::new())
    //         .unwrap();

    //     let mut buf = Vec::new();
    //     request.record_raw(&mut buf).await.unwrap();
    //     assert_eq!(buf, b"GET http://localhost:3000/ HTTP/1.1\n\n");
    // }

    // #[tokio::test]
    // async fn test_empty_body_response() {
    //     let response =
    //         http::Response::builder().status(http::StatusCode::OK).body(http_body_util::Empty::<Bytes>::new()).unwrap();

    //     let mut buf = Vec::new();
    //     response.record_raw(&mut buf).await.unwrap();
    //     assert_eq!(buf, b"HTTP/1.1 200 OK\n\n");
    // }
}
