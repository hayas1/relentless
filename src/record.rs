use std::{fs::File, path::Path};

use http_body::Body;
use http_body_util::{BodyExt, Collected};

#[allow(async_fn_in_trait)] // TODO #[warn(async_fn_in_trait)] by default
pub trait Recordable {
    type Error;
    async fn record_raw<W: std::io::Write>(&self, w: &mut W) -> Result<(), Self::Error>;
    async fn record<W: std::io::Write>(&self, w: &mut W) -> Result<(), Self::Error> {
        self.record_raw(w).await
    }

    async fn record_file(&self, file: &mut File) -> Result<(), Self::Error> {
        self.record(file).await
    }
    async fn record_file_raw(&self, file: &mut File) -> Result<(), Self::Error> {
        self.record_raw(file).await
    }

    async fn record_path_raw<P>(&self, path: P) -> Result<(), Self::Error>
    where
        P: AsRef<Path>,
        Self::Error: From<std::io::Error>,
    {
        self.record_file_raw(&mut File::create(path.as_ref())?).await
    }
    async fn record_path<P>(&self, path: P) -> Result<(), Self::Error>
    where
        P: AsRef<Path>,
        Self::Error: From<std::io::Error>,
    {
        self.record_file(&mut File::create(path.as_ref())?).await
    }
}

impl<B> Recordable for http::Request<B>
where
    B: Body + Clone,
{
    type Error = std::io::Error;
    async fn record<W: std::io::Write>(&self, _w: &mut W) -> Result<(), Self::Error> {
        // TODO from content-type
        unimplemented!("json");
    }
    async fn record_raw<W: std::io::Write>(&self, w: &mut W) -> Result<(), Self::Error> {
        let (method, uri, version) = (self.method(), self.uri(), self.version());
        writeln!(w, "{} {} {:?}", method, uri, version)?;

        let headers = self.headers();
        for (header, value) in headers.iter() {
            writeln!(w, "{}: {:?}", header, value)?;
        }
        writeln!(w)?;

        let body = self.body().clone();
        if let Ok(b) = BodyExt::collect(body).await.map(Collected::to_bytes) {
            write!(w, "{}", String::from_utf8_lossy(&b))?;
        }

        Ok(())
    }
}

impl<B> Recordable for http::Response<B>
where
    B: Body + Clone,
{
    type Error = std::io::Error;
    async fn record<W: std::io::Write>(&self, _w: &mut W) -> Result<(), Self::Error> {
        // TODO from content-type
        unimplemented!("json");
    }
    async fn record_raw<W: std::io::Write>(&self, w: &mut W) -> Result<(), Self::Error> {
        let (version, status) = (self.version(), self.status());
        writeln!(w, "{:?} {}", version, status)?;

        let headers = self.headers();
        for (header, value) in headers.iter() {
            writeln!(w, "{}: {:?}", header, value)?;
        }
        writeln!(w)?;

        let body = self.body().clone();
        if let Ok(b) = BodyExt::collect(body).await.map(Collected::to_bytes) {
            write!(w, "{}", String::from_utf8_lossy(&b))?;
        }

        Ok(())
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
