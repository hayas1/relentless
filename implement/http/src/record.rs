// use std::path::PathBuf;

// use bytes::Bytes;
// use http::header::CONTENT_TYPE;
// use http_body::Body;
// use http_body_util::{BodyExt, Collected};
// use relentless::assault::service::record::{CloneCollected, Recordable, RecordableRequest};

// impl<B> Recordable for http::Request<B>
// where
//     B: Body + Send,
//     B::Data: Send,
// {
//     type Error = std::io::Error;
//     fn extension(&self) -> &'static str {
//         if let Some(content_type) = self.headers().get(CONTENT_TYPE) {
//             if content_type == mime::APPLICATION_JSON.as_ref() {
//                 "json"
//             } else {
//                 "txt"
//             }
//         } else {
//             "txt"
//         }
//     }
//     async fn record<W: std::io::Write>(self, w: &mut W) -> Result<(), Self::Error> {
//         let body = BodyExt::collect(self.into_body()).await.map(Collected::to_bytes).unwrap_or_default();
//         write!(w, "{}", String::from_utf8_lossy(&body))
//     }
//     async fn record_raw<W: std::io::Write>(self, w: &mut W) -> Result<(), Self::Error> {
//         let (http::request::Parts { method, uri, version, headers, .. }, body) = self.into_parts();

//         writeln!(w, "{} {} {:?}", method, uri, version)?;
//         for (header, value) in headers.iter() {
//             writeln!(w, "{}: {:?}", header, value)?;
//         }
//         writeln!(w)?;
//         if let Ok(b) = BodyExt::collect(body).await.map(Collected::to_bytes) {
//             write!(w, "{}", String::from_utf8_lossy(&b))?;
//         }

//         Ok(())
//     }
// }

// impl<B> CloneCollected for http::Request<B>
// where
//     B: Body + From<Bytes> + Send,
//     B::Data: Send,
// {
//     type CollectError = B::Error;
//     async fn clone_collected(self) -> Result<(Self, Self), Self::CollectError> {
//         let (req_parts, req_body) = self.into_parts();
//         let req_bytes = BodyExt::collect(req_body).await.map(Collected::to_bytes)?;
//         let req1 = http::Request::from_parts(req_parts.clone(), B::from(req_bytes.clone()));
//         let req2 = http::Request::from_parts(req_parts, B::from(req_bytes));
//         Ok((req1, req2))
//     }
// }
// impl<B> RecordableRequest for http::Request<B> {
//     fn record_dir(&self) -> PathBuf {
//         self.uri().to_string().into()
//     }
// }

// impl<B> Recordable for http::Response<B>
// where
//     B: Body + Send,
//     B::Data: Send,
// {
//     type Error = std::io::Error;
//     fn extension(&self) -> &'static str {
//         if let Some(content_type) = self.headers().get(CONTENT_TYPE) {
//             if content_type == mime::APPLICATION_JSON.as_ref() {
//                 "json"
//             } else {
//                 "txt"
//             }
//         } else {
//             "txt"
//         }
//     }
//     async fn record<W: std::io::Write>(self, w: &mut W) -> Result<(), Self::Error> {
//         let body = BodyExt::collect(self.into_body()).await.map(Collected::to_bytes).unwrap_or_default();
//         write!(w, "{}", String::from_utf8_lossy(&body))
//     }
//     async fn record_raw<W: std::io::Write>(self, w: &mut W) -> Result<(), Self::Error> {
//         let (http::response::Parts { version, status, headers, .. }, body) = self.into_parts();

//         writeln!(w, "{:?} {}", version, status)?;
//         for (header, value) in headers.iter() {
//             writeln!(w, "{}: {:?}", header, value)?;
//         }
//         writeln!(w)?;
//         if let Ok(b) = BodyExt::collect(body).await.map(Collected::to_bytes) {
//             write!(w, "{}", String::from_utf8_lossy(&b))?;
//         }

//         Ok(())
//     }
// }
// impl<B> CloneCollected for http::Response<B>
// where
//     B: Body + From<Bytes> + Send,
//     B::Data: Send,
// {
//     type CollectError = B::Error;
//     async fn clone_collected(self) -> Result<(Self, Self), Self::CollectError> {
//         // once consume body to record, and reconstruct to response
//         let (res_parts, res_body) = self.into_parts();
//         let res_bytes = BodyExt::collect(res_body).await.map(Collected::to_bytes)?;
//         let res1 = http::Response::from_parts(res_parts.clone(), B::from(res_bytes.clone()));
//         let res2 = http::Response::from_parts(res_parts, B::from(res_bytes));
//         Ok((res1, res2))
//     }
// }
