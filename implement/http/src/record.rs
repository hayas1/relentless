use bytes::Bytes;
use http::header::CONTENT_TYPE;
use http_body::Body;
use http_body_util::{BodyExt, Collected};
use relentless::assault::service::record::{CollectClone, IoRecord, RequestIoRecord};

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Default, Hash)]
pub struct HttpIoRecorder;

impl<B> IoRecord<http::Request<B>> for HttpIoRecorder
where
    B: Body + From<Bytes> + Send,
    B::Data: Send,
{
    type Error = std::io::Error;
    fn extension(&self, r: &http::Request<B>) -> &'static str {
        if let Some(content_type) = r.headers().get(CONTENT_TYPE) {
            if content_type == mime::APPLICATION_JSON.as_ref() {
                "json"
            } else {
                "txt"
            }
        } else {
            "txt"
        }
    }
    async fn record<W: std::io::Write + Send>(&self, w: &mut W, r: http::Request<B>) -> Result<(), Self::Error> {
        let body = BodyExt::collect(r.into_body()).await.map(Collected::to_bytes).unwrap_or_default();
        write!(w, "{}", String::from_utf8_lossy(&body))
    }
    async fn record_raw<W: std::io::Write + Send>(&self, w: &mut W, r: http::Request<B>) -> Result<(), Self::Error> {
        let (http::request::Parts { method, uri, version, headers, .. }, body) = r.into_parts();

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

impl<B> CollectClone<http::Request<B>> for HttpIoRecorder
where
    B: Body + From<Bytes> + Send,
    B::Data: Send,
{
    type Error = B::Error;
    async fn collect_clone(&self, r: http::Request<B>) -> Result<(http::Request<B>, http::Request<B>), Self::Error> {
        // once consume body to record, and reconstruct to request
        let (req_parts, req_body) = r.into_parts();
        let req_bytes = BodyExt::collect(req_body).await.map(Collected::to_bytes)?;
        let req1 = http::Request::from_parts(req_parts.clone(), B::from(req_bytes.clone()));
        let req2 = http::Request::from_parts(req_parts, B::from(req_bytes));
        Ok((req1, req2))
    }
}
impl<B> RequestIoRecord<http::Request<B>> for HttpIoRecorder {
    fn record_dir(&self, r: &http::Request<B>) -> std::path::PathBuf {
        r.uri().to_string().into()
    }
}

impl<B> IoRecord<http::Response<B>> for HttpIoRecorder
where
    B: Body + From<Bytes> + Send,
    B::Data: Send,
{
    type Error = std::io::Error;
    fn extension(&self, r: &http::Response<B>) -> &'static str {
        if let Some(content_type) = r.headers().get(CONTENT_TYPE) {
            if content_type == mime::APPLICATION_JSON.as_ref() {
                "json"
            } else {
                "txt"
            }
        } else {
            "txt"
        }
    }
    async fn record<W: std::io::Write>(&self, w: &mut W, r: http::Response<B>) -> Result<(), Self::Error> {
        let body = BodyExt::collect(r.into_body()).await.map(Collected::to_bytes).unwrap_or_default();
        write!(w, "{}", String::from_utf8_lossy(&body))
    }

    async fn record_raw<W: std::io::Write>(&self, w: &mut W, r: http::Response<B>) -> Result<(), Self::Error> {
        let (http::response::Parts { version, status, headers, .. }, body) = r.into_parts();

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
impl<B> CollectClone<http::Response<B>> for HttpIoRecorder
where
    B: Body + From<Bytes> + Send,
    B::Data: Send,
{
    type Error = B::Error;
    async fn collect_clone(&self, r: http::Response<B>) -> Result<(http::Response<B>, http::Response<B>), Self::Error> {
        // once consume body to record, and reconstruct to response
        let (res_parts, res_body) = r.into_parts();
        let res_bytes = BodyExt::collect(res_body).await.map(Collected::to_bytes)?;
        let res1 = http::Response::from_parts(res_parts.clone(), B::from(res_bytes.clone()));
        let res2 = http::Response::from_parts(res_parts, B::from(res_bytes));
        Ok((res1, res2))
    }
}
