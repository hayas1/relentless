// use std::path::PathBuf;

// use bytes::Bytes;
// use relentless::assault::service::record::{CloneCollected, Recordable, RecordableRequest};
// use serde::{de::DeserializeOwned, Deserialize};

// use crate::client::DefaultGrpcRequest;

// impl<De, Se> Recordable for DefaultGrpcRequest<De, Se>
// where
//     De: for<'a> serde::Deserializer<'a> + Send + Sync + 'static,
//     for<'a> <De as serde::Deserializer<'a>>::Error: std::error::Error + Send + Sync + 'static,
//     Se: Send,
// {
//     type Error = std::io::Error;
//     fn extension(&self) -> &'static str {
//         "json"
//     }
//     async fn record<W: std::io::Write>(self, w: &mut W) -> Result<(), Self::Error> {
//         let value = serde_json::Value::deserialize(self.message)
//             .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
//         write!(w, "{}", serde_json::to_string_pretty(&value).unwrap())
//     }
//     async fn record_raw<W: std::io::Write + Send>(self, w: &mut W) -> Result<(), Self::Error> {
//         let uri = self.destination;
//         let (metadata, extension, message) = tonic::Request::new(self.message).into_parts();
//         let mut http_request_builder =
//             http::Request::builder().method(http::Method::POST).uri(uri).extension(extension);
//         if let Some(headers) = http_request_builder.headers_mut() {
//             *headers = metadata.into_headers();
//         }
//         let body = Bytes::from(
//             serde_json::to_vec(
//                 &serde_json::Value::deserialize(message)
//                     .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?,
//             )
//             .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?,
//         );
//         let http_request = http_request_builder
//             .body(http_body_util::Full::new(body))
//             .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;

//         http_request.record_raw(w).await
//     }
// }
// impl<De, Se> CloneCollected for DefaultGrpcRequest<De, Se>
// where
//     De: for<'a> serde::Deserializer<'a> + DeserializeOwned + Send + Sync + 'static,
//     for<'a> <De as serde::Deserializer<'a>>::Error: std::error::Error + Send + Sync + 'static,
//     Se: Send,
// {
//     type CollectError = std::io::Error;
//     async fn clone_collected(self) -> Result<(Self, Self), Self::CollectError> {
//         let Self { destination, service, method, codec, message } = self;
//         let value = serde_json::Value::deserialize(message)
//             .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
//         let m1 = serde_json::from_value(value.clone())
//             .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
//         let m2 = serde_json::from_value(value).map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
//         Ok((
//             Self {
//                 destination: destination.clone(),
//                 service: service.clone(),
//                 method: method.clone(),
//                 codec: codec.clone(),
//                 message: m1,
//             },
//             Self { destination, service, method, codec, message: m2 },
//         ))
//     }
// }
// impl<De, Se> RecordableRequest for DefaultGrpcRequest<De, Se> {
//     fn record_dir(&self) -> PathBuf {
//         http::uri::Builder::from(self.destination.clone())
//             .path_and_query(self.format_method_path())
//             .build()
//             .unwrap_or_else(|e| unreachable!("{}", e))
//             .to_string()
//             .into()
//     }
// }
// impl Recordable for tonic::Response<<serde_json::value::Serializer as serde::Serializer>::Ok> {
//     type Error = std::io::Error;
//     fn extension(&self) -> &'static str {
//         "json"
//     }
//     async fn record<W: std::io::Write>(self, w: &mut W) -> Result<(), Self::Error> {
//         let value = serde_json::Value::deserialize(self.into_inner())
//             .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
//         write!(w, "{}", serde_json::to_string_pretty(&value).unwrap())
//     }
//     async fn record_raw<W: std::io::Write + Send>(self, w: &mut W) -> Result<(), Self::Error> {
//         let (metadata, message, extension) = self.into_parts();
//         let mut http_response_builder = http::Response::builder().extension(extension);
//         if let Some(headers) = http_response_builder.headers_mut() {
//             *headers = metadata.into_headers();
//         }
//         let body = Bytes::from(
//             serde_json::to_vec(
//                 &serde_json::Value::deserialize(message)
//                     .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?,
//             )
//             .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?,
//         );
//         let http_response = http_response_builder
//             .body(http_body_util::Full::new(body))
//             .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;

//         http_response.record_raw(w).await
//     }
// }
// impl CloneCollected for tonic::Response<<serde_json::value::Serializer as serde::Serializer>::Ok> {
//     type CollectError = std::io::Error;
//     async fn clone_collected(self) -> Result<(Self, Self), Self::CollectError> {
//         let (metadata, message, extension) = self.into_parts();
//         let value = serde_json::Value::deserialize(message)
//             .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
//         let m1 = serde_json::from_value(value.clone())
//             .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
//         let m2 = serde_json::from_value(value).map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
//         Ok((Self::from_parts(metadata.clone(), m1, extension.clone()), Self::from_parts(metadata, m2, extension)))
//     }
// }
