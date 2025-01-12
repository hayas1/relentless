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
use http_body::Body;
use http_body_util::{BodyExt, Collected};
use serde::{de::DeserializeOwned, Deserialize};
use tower::{Layer, Service};

use crate::error::IntoResult;
#[cfg(feature = "json")]
use crate::implement::service_grpc::client::DefaultGrpcRequest;

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

pub mod http_record {
    use super::*;

    impl<B> Recordable for http::Request<B>
    where
        B: Body + Send,
        B::Data: Send,
    {
        type Error = std::io::Error;
        fn extension(&self) -> &'static str {
            if let Some(content_type) = self.headers().get(CONTENT_TYPE) {
                if content_type == mime::APPLICATION_JSON.as_ref() {
                    "json"
                } else {
                    "txt"
                }
            } else {
                "txt"
            }
        }
        async fn record<W: std::io::Write>(self, w: &mut W) -> Result<(), Self::Error> {
            let body = BodyExt::collect(self.into_body()).await.map(Collected::to_bytes).unwrap_or_default();
            write!(w, "{}", String::from_utf8_lossy(&body))
        }
        async fn record_raw<W: std::io::Write>(self, w: &mut W) -> Result<(), Self::Error> {
            let (http::request::Parts { method, uri, version, headers, .. }, body) = self.into_parts();

            writeln!(w, "{} {} {:?}", method, uri, version)?;
            for (header, value) in headers.iter() {
                writeln!(w, "{}: {:?}", header, value)?;
            }
            writeln!(w)?;
            if let Ok(b) = BodyExt::collect(body).await.map(Collected::to_bytes) {
                write!(w, "{}", String::from_utf8_lossy(&b))?;
            }

            Ok(())
        }
    }

    impl<B> CloneCollected for http::Request<B>
    where
        B: Body + From<Bytes> + Send,
        B::Data: Send,
    {
        type CollectError = B::Error;
        async fn clone_collected(self) -> Result<(Self, Self), Self::CollectError> {
            let (req_parts, req_body) = self.into_parts();
            let req_bytes = BodyExt::collect(req_body).await.map(Collected::to_bytes)?;
            let req1 = http::Request::from_parts(req_parts.clone(), B::from(req_bytes.clone()));
            let req2 = http::Request::from_parts(req_parts, B::from(req_bytes));
            Ok((req1, req2))
        }
    }
    impl<B> RecordableRequest for http::Request<B> {
        fn record_dir(&self) -> PathBuf {
            self.uri().to_string().into()
        }
    }

    impl<B> Recordable for http::Response<B>
    where
        B: Body + Send,
        B::Data: Send,
    {
        type Error = std::io::Error;
        fn extension(&self) -> &'static str {
            if let Some(content_type) = self.headers().get(CONTENT_TYPE) {
                if content_type == mime::APPLICATION_JSON.as_ref() {
                    "json"
                } else {
                    "txt"
                }
            } else {
                "txt"
            }
        }
        async fn record<W: std::io::Write>(self, w: &mut W) -> Result<(), Self::Error> {
            let body = BodyExt::collect(self.into_body()).await.map(Collected::to_bytes).unwrap_or_default();
            write!(w, "{}", String::from_utf8_lossy(&body))
        }
        async fn record_raw<W: std::io::Write>(self, w: &mut W) -> Result<(), Self::Error> {
            let (http::response::Parts { version, status, headers, .. }, body) = self.into_parts();

            writeln!(w, "{:?} {}", version, status)?;
            for (header, value) in headers.iter() {
                writeln!(w, "{}: {:?}", header, value)?;
            }
            writeln!(w)?;
            if let Ok(b) = BodyExt::collect(body).await.map(Collected::to_bytes) {
                write!(w, "{}", String::from_utf8_lossy(&b))?;
            }

            Ok(())
        }
    }
    impl<B> CloneCollected for http::Response<B>
    where
        B: Body + From<Bytes> + Send,
        B::Data: Send,
    {
        type CollectError = B::Error;
        async fn clone_collected(self) -> Result<(Self, Self), Self::CollectError> {
            // once consume body to record, and reconstruct to response
            let (res_parts, res_body) = self.into_parts();
            let res_bytes = BodyExt::collect(res_body).await.map(Collected::to_bytes)?;
            let res1 = http::Response::from_parts(res_parts.clone(), B::from(res_bytes.clone()));
            let res2 = http::Response::from_parts(res_parts, B::from(res_bytes));
            Ok((res1, res2))
        }
    }
}

#[cfg(feature = "json")]
pub mod grpc_record {
    use super::*;

    impl<De, Se> Recordable for DefaultGrpcRequest<De, Se>
    where
        De: for<'a> serde::Deserializer<'a> + Send + Sync + 'static,
        for<'a> <De as serde::Deserializer<'a>>::Error: std::error::Error + Send + Sync + 'static,
        Se: Send,
    {
        type Error = std::io::Error;
        fn extension(&self) -> &'static str {
            "json"
        }
        async fn record<W: std::io::Write>(self, w: &mut W) -> Result<(), Self::Error> {
            let value = serde_json::Value::deserialize(self.message)
                .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
            write!(w, "{}", serde_json::to_string_pretty(&value).unwrap())
        }
        async fn record_raw<W: std::io::Write + Send>(self, w: &mut W) -> Result<(), Self::Error> {
            let uri = self.destination;
            let (metadata, extension, message) = tonic::Request::new(self.message).into_parts();
            let mut http_request_builder = http::Request::builder().method(Method::POST).uri(uri).extension(extension);
            if let Some(headers) = http_request_builder.headers_mut() {
                *headers = metadata.into_headers();
            }
            let body = Bytes::from(
                serde_json::to_vec(
                    &serde_json::Value::deserialize(message)
                        .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?,
                )
                .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?,
            );
            let http_request = http_request_builder
                .body(http_body_util::Full::new(body))
                .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;

            http_request.record_raw(w).await
        }
    }
    impl<De, Se> CloneCollected for DefaultGrpcRequest<De, Se>
    where
        De: for<'a> serde::Deserializer<'a> + DeserializeOwned + Send + Sync + 'static,
        for<'a> <De as serde::Deserializer<'a>>::Error: std::error::Error + Send + Sync + 'static,
        Se: Send,
    {
        type CollectError = std::io::Error;
        async fn clone_collected(self) -> Result<(Self, Self), Self::CollectError> {
            let Self { destination, service, method, codec, message } = self;
            let value = serde_json::Value::deserialize(message)
                .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
            let m1 = serde_json::from_value(value.clone())
                .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
            let m2 =
                serde_json::from_value(value).map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
            Ok((
                Self {
                    destination: destination.clone(),
                    service: service.clone(),
                    method: method.clone(),
                    codec: codec.clone(),
                    message: m1,
                },
                Self { destination, service, method, codec, message: m2 },
            ))
        }
    }
    impl<De, Se> RecordableRequest for DefaultGrpcRequest<De, Se> {
        fn record_dir(&self) -> PathBuf {
            http::uri::Builder::from(self.destination.clone())
                .path_and_query(self.format_method_path())
                .build()
                .unwrap_or_else(|e| unreachable!("{}", e))
                .to_string()
                .into()
        }
    }
    impl Recordable for tonic::Response<<serde_json::value::Serializer as serde::Serializer>::Ok> {
        type Error = std::io::Error;
        fn extension(&self) -> &'static str {
            "json"
        }
        async fn record<W: std::io::Write>(self, w: &mut W) -> Result<(), Self::Error> {
            let value = serde_json::Value::deserialize(self.into_inner())
                .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
            write!(w, "{}", serde_json::to_string_pretty(&value).unwrap())
        }
        async fn record_raw<W: std::io::Write + Send>(self, w: &mut W) -> Result<(), Self::Error> {
            let (metadata, message, extension) = self.into_parts();
            let mut http_response_builder = http::Response::builder().extension(extension);
            if let Some(headers) = http_response_builder.headers_mut() {
                *headers = metadata.into_headers();
            }
            let body = Bytes::from(
                serde_json::to_vec(
                    &serde_json::Value::deserialize(message)
                        .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?,
                )
                .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?,
            );
            let http_response = http_response_builder
                .body(http_body_util::Full::new(body))
                .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;

            http_response.record_raw(w).await
        }
    }
    impl CloneCollected for tonic::Response<<serde_json::value::Serializer as serde::Serializer>::Ok> {
        type CollectError = std::io::Error;
        async fn clone_collected(self) -> Result<(Self, Self), Self::CollectError> {
            let (metadata, message, extension) = self.into_parts();
            let value = serde_json::Value::deserialize(message)
                .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
            let m1 = serde_json::from_value(value.clone())
                .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
            let m2 =
                serde_json::from_value(value).map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
            Ok((Self::from_parts(metadata.clone(), m1, extension.clone()), Self::from_parts(metadata, m2, extension)))
        }
    }
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
    use bytes::Bytes;
    use http::Method;

    use super::*;

    #[tokio::test]
    async fn test_empty_body_request() {
        let request = http::Request::builder()
            .method(Method::GET)
            .uri("http://localhost:3000")
            .body(http_body_util::Empty::<Bytes>::new())
            .unwrap();

        let mut buf = Vec::new();
        request.record_raw(&mut buf).await.unwrap();
        assert_eq!(buf, b"GET http://localhost:3000/ HTTP/1.1\n\n");
    }

    #[tokio::test]
    async fn test_empty_body_response() {
        let response =
            http::Response::builder().status(http::StatusCode::OK).body(http_body_util::Empty::<Bytes>::new()).unwrap();

        let mut buf = Vec::new();
        response.record_raw(&mut buf).await.unwrap();
        assert_eq!(buf, b"HTTP/1.1 200 OK\n\n");
    }
}
