use prost_types::{Any, Value};
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

    impl MetadataMap {
        pub fn to_map(&self) -> std::collections::HashMap<String, String> {
            self.entries
                .iter()
                .filter_map(|e| {
                    Some(match e.entry.as_ref()? {
                        map_entry::Entry::Ascii(ascii) => (ascii.key.to_string(), ascii.value.to_string()),
                        map_entry::Entry::Binary(binary) => (
                            String::from_utf8_lossy(&binary.key).to_string(),
                            String::from_utf8_lossy(&binary.value).to_string(),
                        ),
                    })
                })
                .collect()
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

#[cfg(test)]
mod tests {
    use pb::echo_server::Echo;

    use super::*;

    #[tokio::test]
    async fn test_echo() {
        let echo = EchoImpl;

        let request = Request::new(Any::from_msg(&"100".to_string()).unwrap());
        let response = echo.echo(request).await.unwrap();
        assert_eq!(response.into_inner().to_msg::<String>().unwrap(), "100");
    }

    #[tokio::test]
    async fn test_echo_value() {
        let echo = EchoImpl;

        let request = Request::new(Value::from(200));
        let response = echo.echo_value(request).await.unwrap();
        assert_eq!(response.into_inner(), Value::from(200));
    }

    #[tokio::test]
    async fn test_echo_metadata() {
        let echo = EchoImpl;

        let mut request = Request::new(());
        request.set_timeout(std::time::Duration::from_secs(1));
        let response = echo.echo_metadata(request).await.unwrap().into_inner();
        assert_eq!(
            response,
            pb::MetadataMap {
                entries: vec![pb::MapEntry {
                    entry: Some(pb::map_entry::Entry::Ascii(pb::AsciiEntry {
                        key: "grpc-timeout".to_string(),
                        value: "1000000u".to_string()
                    }))
                }]
            }
        );
        assert_eq!(response.to_map(), vec![("grpc-timeout".to_string(), "1000000u".to_string())].into_iter().collect());
    }
}
