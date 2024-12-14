use std::{
    fs::File,
    future::Future,
    io::Write,
    path::PathBuf,
    pin::Pin,
    task::{Context, Poll},
};

use bytes::Bytes;
use http::header::CONTENT_TYPE;
use http_body::Body;
use http_body_util::{BodyExt, Collected};
use tower::{Layer, Service};

use crate::error::{Wrap, WrappedResult};

// ##### TODO record to sqlite or duckdb #####
#[allow(async_fn_in_trait)] // TODO #[warn(async_fn_in_trait)] by default
pub trait Recordable: Sized {
    type Error;
    async fn record_raw<W: std::io::Write>(self, w: &mut W) -> Result<(), Self::Error>;
    fn extension(&self) -> &'static str {
        "txt"
    }
    async fn record<W: std::io::Write>(self, w: &mut W) -> Result<(), Self::Error> {
        self.record_raw(w).await
    }
}

impl<B> Recordable for http::Request<B>
where
    B: Body,
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

impl<B> Recordable for http::Response<B>
where
    B: Body,
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

#[derive(Debug, Clone)]
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

#[derive(Debug, Clone)]
pub struct RecordService<S> {
    path: Option<PathBuf>,
    inner: S,
}
impl<S, ReqB, ResB> Service<http::Request<ReqB>> for RecordService<S>
where
    ReqB: Body + From<Bytes> + Send + 'static,
    ReqB::Data: Send,
    ResB: Body + From<Bytes> + Send,
    ResB::Data: Send,
    S: Service<http::Request<ReqB>, Response = http::Response<ResB>> + Clone + Send + 'static,
    S::Future: Send + 'static,
    Wrap: From<S::Error> + From<ReqB::Error> + From<ResB::Error>,
{
    type Response = S::Response;
    type Error = crate::Error;
    type Future = Pin<Box<dyn Future<Output = Result<Self::Response, Self::Error>> + Send>>;

    fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        self.inner.poll_ready(cx).map_err(crate::Error::wrap)
    }

    fn call(&mut self, request: http::Request<ReqB>) -> Self::Future {
        let paths = (|p: Option<&PathBuf>| {
            // TODO path will be uri ... (if implement template, it will not be in path)
            // TODO timestamp or repeated number
            // TODO join path (absolute) https://github.com/rust-lang/rust/issues/16507
            let dir = p?.join(request.uri().to_string());
            std::fs::create_dir_all(&dir).ok()?;
            writeln!(File::create(p?.join(".gitignore")).ok()?, "*").ok()?; // TODO hardcode...
            Some(((dir.join("raw_request"), dir.join("request")), (dir.join("raw_response"), dir.join("response"))))
        })(self.path.as_ref());

        if let Some(((path_raw_req, path_req), (path_raw_res, path_res))) = paths {
            let mut cloned_inner = self.inner.clone();
            Box::pin(async move {
                let response: WrappedResult<_> = async move {
                    // once consume body for record, and reconstruct for request
                    let (req_parts, req_body) = request.into_parts();
                    let req_bytes = BodyExt::collect(req_body).await.map(Collected::to_bytes)?;
                    let recordable_raw_req =
                        http::Request::from_parts(req_parts.clone(), ReqB::from(req_bytes.clone()));
                    recordable_raw_req.record_raw(&mut File::create(path_raw_req.with_extension("txt"))?).await?;
                    let recordable_req = http::Request::from_parts(req_parts.clone(), ReqB::from(req_bytes.clone()));
                    let req_record_extension = recordable_req.extension();
                    recordable_req.record(&mut File::create(path_req.with_extension(req_record_extension))?).await?;
                    let req = http::Request::from_parts(req_parts, ReqB::from(req_bytes));

                    // once consume body for record, and reconstruct for response
                    let res = cloned_inner.call(req).await?;
                    let (res_parts, res_body) = res.into_parts();
                    let res_bytes = BodyExt::collect(res_body).await.map(Collected::to_bytes)?;
                    let recordable_raw_res =
                        http::Response::from_parts(res_parts.clone(), ResB::from(res_bytes.clone()));
                    recordable_raw_res.record_raw(&mut File::create(path_raw_res.with_extension("txt"))?).await?;
                    let recordable_res = http::Response::from_parts(res_parts.clone(), ReqB::from(res_bytes.clone()));
                    let res_record_extension = recordable_res.extension();
                    recordable_res.record(&mut File::create(path_res.with_extension(res_record_extension))?).await?;
                    Ok(http::Response::from_parts(res_parts, ResB::from(res_bytes)))
                }
                .await;
                Ok(response?)
            })
        } else {
            let fut = self.inner.call(request);
            Box::pin(async move { fut.await.map_err(crate::Error::wrap) })
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
