use prost_types::{Any, Value};
use thiserror::Error;
use tonic::{Request, Response, Status};

pub mod pb {
    tonic::include_proto!("echo");

    impl From<tonic::metadata::MetadataMap> for MetadataMap {
        fn from(map: tonic::metadata::MetadataMap) -> Self {
            Self::from(&map)
        }
    }
    impl From<&tonic::metadata::MetadataMap> for MetadataMap {
        fn from(map: &tonic::metadata::MetadataMap) -> Self {
            Self {
                entries: map
                    .iter()
                    .map(|kv| MapEntry {
                        entry: Some(match kv {
                            tonic::metadata::KeyAndValueRef::Ascii(k, v) => map_entry::Entry::Ascii(AsciiEntry {
                                key: k.to_string(),
                                value: String::from_utf8_lossy(v.as_encoded_bytes()).to_string(),
                            }),
                            tonic::metadata::KeyAndValueRef::Binary(k, v) => map_entry::Entry::Binary(BinaryEntry {
                                key: <tonic::metadata::MetadataKey<tonic::metadata::Binary> as std::convert::AsRef<
                                    [u8],
                                >>::as_ref(k)
                                .to_vec(),
                                value: v.as_encoded_bytes().to_vec(),
                            }),
                        }),
                    })
                    .collect(),
            }
        }
    }
    impl TryFrom<MetadataMap> for tonic::metadata::MetadataMap {
        type Error = super::EchoError;
        fn try_from(map: MetadataMap) -> Result<Self, Self::Error> {
            let mut metadata_map = tonic::metadata::MetadataMap::new();
            for e in map.entries {
                match e.entry.ok_or(super::EchoError::NoEntry)? {
                    map_entry::Entry::Ascii(ascii) => metadata_map.append(
                        ascii.key.parse::<tonic::metadata::MetadataKey<_>>()?,
                        ascii.value.parse::<tonic::metadata::MetadataValue<_>>()?,
                    ),
                    map_entry::Entry::Binary(binary) => metadata_map.append_bin(
                        tonic::metadata::MetadataKey::from_bytes(&binary.key)?,
                        tonic::metadata::MetadataValue::from_bytes(&binary.value),
                    ),
                };
            }
            Ok(metadata_map)
        }
    }
}

#[derive(Debug, Default)]
pub struct EchoImpl;

#[tonic::async_trait]
impl pb::echo_server::Echo for EchoImpl {
    #[tracing::instrument(ret)]
    async fn echo(&self, request: Request<Any>) -> Result<Response<Any>, Status> {
        let value = request.into_inner();
        Ok(Response::new(value))
    }
    #[tracing::instrument(ret)]
    async fn echo_value(&self, request: Request<Value>) -> Result<Response<Value>, Status> {
        let value = request.into_inner();
        Ok(Response::new(value))
    }
    #[tracing::instrument(ret)]
    async fn echo_metadata(&self, request: Request<()>) -> Result<Response<pb::MetadataMap>, Status> {
        let map = request.metadata();
        Ok(Response::new(pb::MetadataMap::from(map)))
    }
}

#[derive(Error, Debug)]
pub enum EchoError {
    #[error(transparent)]
    MetadataKeyError(#[from] tonic::metadata::errors::InvalidMetadataKey),
    #[error(transparent)]
    MetadataValueError(#[from] tonic::metadata::errors::InvalidMetadataValue),
    #[error("no entry in metadata")]
    NoEntry,
}

#[cfg(test)]
mod tests {
    use pb::{echo_client::EchoClient, echo_server::EchoServer};

    use super::*;

    #[tokio::test]
    async fn test_echo() {
        let server = EchoServer::new(EchoImpl);
        let mut client = EchoClient::new(server);

        let request = Request::new(Any::from_msg(&"100".to_string()).unwrap());
        let response = client.echo(request).await.unwrap();
        assert_eq!(response.into_inner().to_msg::<String>().unwrap(), "100");
    }

    #[tokio::test]
    async fn test_echo_value() {
        let server = EchoServer::new(EchoImpl);
        let mut client = EchoClient::new(server);

        let request = Request::new(Value::from(200));
        let response = client.echo_value(request).await.unwrap();
        assert_eq!(response.into_inner(), Value::from(200));
    }

    #[tokio::test]
    async fn test_echo_metadata() {
        let server = EchoServer::new(EchoImpl);
        let mut client = EchoClient::new(server);

        let mut request = Request::new(());
        request.set_timeout(std::time::Duration::from_secs(1));
        let response = client.echo_metadata(request).await.unwrap().into_inner();
        assert_eq!(
            response,
            pb::MetadataMap {
                entries: vec![
                    pb::MapEntry {
                        entry: Some(pb::map_entry::Entry::Ascii(pb::AsciiEntry {
                            key: "grpc-timeout".to_string(),
                            value: "1000000u".to_string()
                        }))
                    },
                    pb::MapEntry {
                        entry: Some(pb::map_entry::Entry::Ascii(pb::AsciiEntry {
                            key: "te".to_string(),
                            value: "trailers".to_string()
                        }))
                    },
                    pb::MapEntry {
                        entry: Some(pb::map_entry::Entry::Ascii(pb::AsciiEntry {
                            key: "content-type".to_string(),
                            value: "application/grpc".to_string()
                        }))
                    },
                ]
            }
        );
        let mut metadata_map = tonic::metadata::MetadataMap::new();
        metadata_map.append(
            tonic::metadata::MetadataKey::from_static("grpc-timeout"),
            tonic::metadata::MetadataValue::from_static("1000000u"),
        );
        metadata_map.append(
            tonic::metadata::MetadataKey::from_static("te"),
            tonic::metadata::MetadataValue::from_static("trailers"),
        );
        metadata_map.append(
            tonic::metadata::MetadataKey::from_static("content-type"),
            tonic::metadata::MetadataValue::from_static("application/grpc"),
        );
        assert_eq!(
            tonic::metadata::MetadataMap::try_from(response).unwrap().into_headers(),
            metadata_map.into_headers()
        );
    }
}
