use std::path::PathBuf;

use bytes::Bytes;
use relentless::assault::service::record::{CollectClone, IoRecord, RequestIoRecord};
use serde::{de::DeserializeOwned, Deserialize};

use crate::client::GrpcMethodRequest;

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Default, Hash)]
pub struct GrpcIoRecorder;

impl<De, Se> IoRecord<GrpcMethodRequest<De, Se>> for GrpcIoRecorder
where
    De: for<'a> serde::Deserializer<'a> + Send + Sync + 'static,
    for<'a> <De as serde::Deserializer<'a>>::Error: std::error::Error + Send + Sync + 'static,
    Se: Send,
{
    type Error = std::io::Error;
    fn extension(&self, _r: &GrpcMethodRequest<De, Se>) -> &'static str {
        "json"
    }
    async fn record<W: std::io::Write>(&self, w: &mut W, r: GrpcMethodRequest<De, Se>) -> Result<(), Self::Error> {
        let value = serde_json::Value::deserialize(r.message)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
        write!(w, "{}", serde_json::to_string_pretty(&value).unwrap())
    }
    async fn record_raw<W: std::io::Write + Send>(
        &self,
        w: &mut W,
        r: GrpcMethodRequest<De, Se>,
    ) -> Result<(), Self::Error> {
        let uri = r.destination;
        let (metadata, extension, message) = tonic::Request::new(r.message).into_parts();
        let mut http_request_builder =
            http::Request::builder().method(http::Method::POST).uri(uri).extension(extension);
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

        relentless_http::record::HttpIoRecorder.record_raw(w, http_request).await
    }
}
impl<De, Se> CollectClone<GrpcMethodRequest<De, Se>> for GrpcIoRecorder
where
    De: for<'a> serde::Deserializer<'a> + DeserializeOwned + Send + Sync + 'static,
    for<'a> <De as serde::Deserializer<'a>>::Error: std::error::Error + Send + Sync + 'static,
    Se: Clone + Send,
{
    type Error = std::io::Error;
    async fn collect_clone(
        &self,
        r: GrpcMethodRequest<De, Se>,
    ) -> Result<(GrpcMethodRequest<De, Se>, GrpcMethodRequest<De, Se>), Self::Error> {
        let GrpcMethodRequest { destination, service, method, codec, message } = r;
        let value = serde_json::Value::deserialize(message)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
        let m1 = serde_json::from_value(value.clone())
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
        let m2 = serde_json::from_value(value).map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
        Ok((
            GrpcMethodRequest {
                destination: destination.clone(),
                service: service.clone(),
                method: method.clone(),
                codec: codec.clone(),
                message: m1,
            },
            GrpcMethodRequest { destination, service, method, codec, message: m2 },
        ))
    }
}
impl<De, Se> RequestIoRecord<GrpcMethodRequest<De, Se>> for GrpcIoRecorder {
    fn record_dir(&self, r: &GrpcMethodRequest<De, Se>) -> PathBuf {
        http::uri::Builder::from(r.destination.clone())
            .path_and_query(r.format_method_path())
            .build()
            .unwrap_or_else(|e| unreachable!("{}", e))
            .to_string()
            .into()
    }
}

impl IoRecord<tonic::Response<<serde_json::value::Serializer as serde::Serializer>::Ok>> for GrpcIoRecorder {
    type Error = std::io::Error;

    fn extension(
        &self,
        _r: &tonic::Response<<serde_json::value::Serializer as serde::Serializer>::Ok>,
    ) -> &'static str {
        "json"
    }
    async fn record<W: std::io::Write + Send>(
        &self,
        w: &mut W,
        r: tonic::Response<<serde_json::value::Serializer as serde::Serializer>::Ok>,
    ) -> Result<(), Self::Error> {
        let value = serde_json::Value::deserialize(r.into_inner())
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
        write!(w, "{}", serde_json::to_string_pretty(&value).unwrap())
    }
    async fn record_raw<W: std::io::Write + Send>(
        &self,
        w: &mut W,
        r: tonic::Response<<serde_json::value::Serializer as serde::Serializer>::Ok>,
    ) -> Result<(), Self::Error> {
        let (metadata, message, extension) = r.into_parts();
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

        relentless_http::record::HttpIoRecorder.record_raw(w, http_response).await
    }
}
impl CollectClone<tonic::Response<<serde_json::value::Serializer as serde::Serializer>::Ok>> for GrpcIoRecorder {
    type Error = std::io::Error;
    async fn collect_clone(
        &self,
        r: tonic::Response<<serde_json::value::Serializer as serde::Serializer>::Ok>,
    ) -> Result<
        (
            tonic::Response<<serde_json::value::Serializer as serde::Serializer>::Ok>,
            tonic::Response<<serde_json::value::Serializer as serde::Serializer>::Ok>,
        ),
        Self::Error,
    > {
        let (metadata, message, extension) = r.into_parts();
        let value = serde_json::Value::deserialize(message)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
        let m1 = serde_json::from_value(value.clone())
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
        let m2 = serde_json::from_value(value).map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
        Ok((
            tonic::Response::from_parts(metadata.clone(), m1, extension.clone()),
            tonic::Response::from_parts(metadata, m2, extension),
        ))
    }
}
