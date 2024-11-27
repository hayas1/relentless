use std::{
    fs::File,
    future::Future,
    io::Write,
    path::{Path, PathBuf},
    pin::Pin,
    task::{Context, Poll},
};

use bytes::Bytes;
use http_body::Body;
use http_body_util::{BodyExt, Collected};
use tower::{Layer, Service};

#[allow(async_fn_in_trait)] // TODO #[warn(async_fn_in_trait)] by default
pub trait Recordable: Sized {
    type Error;
    async fn record_raw<W: std::io::Write>(self, w: &mut W) -> Result<(), Self::Error>;
    async fn record<W: std::io::Write>(self, w: &mut W) -> Result<(), Self::Error> {
        self.record_raw(w).await
    }

    async fn record_file(self, file: &mut File) -> Result<(), Self::Error> {
        self.record(file).await
    }
    async fn record_file_raw(self, file: &mut File) -> Result<(), Self::Error> {
        self.record_raw(file).await
    }

    async fn record_path_raw<P>(self, path: P) -> Result<(), Self::Error>
    where
        P: AsRef<Path>,
        Self::Error: From<std::io::Error>,
    {
        self.record_file_raw(&mut File::create(path.as_ref())?).await
    }
    async fn record_path<P>(self, path: P) -> Result<(), Self::Error>
    where
        P: AsRef<Path>,
        Self::Error: From<std::io::Error>,
    {
        self.record_file(&mut File::create(path.as_ref())?).await
    }
}

impl<B> Recordable for http::Request<B>
where
    B: Body,
{
    type Error = std::io::Error;
    async fn record<W: std::io::Write>(self, _w: &mut W) -> Result<(), Self::Error> {
        // TODO from content-type
        unimplemented!("json");
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
    async fn record<W: std::io::Write>(self, _w: &mut W) -> Result<(), Self::Error> {
        // TODO from content-type
        unimplemented!("json");
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
    path: Option<PathBuf>, // TODO do not use Option
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
    ReqB: Body + From<Bytes>,
    ResB: Body + From<Bytes>,
    S: Service<http::Request<ReqB>, Response = http::Response<ResB>>,
    S::Future: 'static,
{
    type Response = S::Response; // TODO S::Response ?
    type Error = S::Error;
    type Future = Pin<Box<dyn Future<Output = Result<Self::Response, Self::Error>>>>;

    fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        self.inner.poll_ready(cx)
    }
    fn call(&mut self, request: http::Request<ReqB>) -> Self::Future {
        // TODO error handling
        let (_path_req, path_res) = if let Some(output) = &self.path {
            // TODO path will be uri ... (if implement template, it will not be in path)
            // TODO timestamp or repeated number
            // TODO join path (absolute) https://github.com/rust-lang/rust/issues/16507
            let dir = output.join(request.uri().to_string());
            std::fs::create_dir_all(&dir).unwrap();
            writeln!(File::create(dir.join(".gitignore")).unwrap(), "*").unwrap();
            (dir.join("request.txt"), dir.join("response.txt"))
        } else {
            // TODO hard coded ...
            (PathBuf::from("/dev/null"), PathBuf::from("/dev/null"))
        };

        // TODO record request

        let fut = self.inner.call(request);
        Box::pin(async move {
            let response = fut.await?;
            let (parts, body) = response.into_parts();
            let bytes = BodyExt::collect(body).await.map(Collected::to_bytes).unwrap_or_else(|_| todo!());
            let record = http::Response::from_parts(parts.clone(), ResB::from(bytes.clone()));
            record.record_path_raw(&path_res).await.unwrap(); // TODO error handling
            let resp = http::Response::from_parts(parts, ResB::from(bytes));
            Ok(resp)
        })
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
