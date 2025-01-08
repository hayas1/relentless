use std::{
    collections::HashMap,
    fs::File,
    io::{BufReader, Read},
    path::PathBuf,
};

use bytes::{Bytes, BytesMut};
use futures::StreamExt;
use http::uri::PathAndQuery;
use prost_reflect::{DescriptorPool, DynamicMessage, MessageDescriptor, MethodDescriptor, ServiceDescriptor};
use prost_types::Value;
use serde::{Deserialize, Serialize};
use tonic::transport::Channel;
use tonic_reflection::pb::v1::{
    server_reflection_client::ServerReflectionClient, server_reflection_request::MessageRequest,
    server_reflection_response::MessageResponse, ServerReflectionRequest, FILE_DESCRIPTOR_SET,
};

use crate::{
    assault::factory::RequestFactory,
    error::IntoResult,
    interface::{
        helper::{coalesce::Coalesce, is_default::IsDefault},
        template::Template,
    },
};

use super::{
    client::{DefaultGrpcRequest, MethodCodec},
    error::GrpcRequestError,
};

#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "kebab-case")]
pub struct GrpcRequest {
    #[serde(default, skip_serializing_if = "IsDefault::is_default")]
    #[cfg_attr(feature = "yaml", serde(with = "serde_yaml::with::singleton_map_recursive"))]
    descriptor: Option<PathBuf>, // TODO compile `.proto` files with `protoc`
    #[serde(default, skip_serializing_if = "IsDefault::is_default")]
    #[cfg_attr(feature = "yaml", serde(with = "serde_yaml::with::singleton_map_recursive"))]
    message: Option<GrpcMessage>,
}
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "kebab-case")]
pub enum GrpcMessage {
    #[default]
    Empty,
    Plaintext(String),
    #[cfg(feature = "json")]
    Json(serde_json::Value),
}
impl Coalesce for GrpcRequest {
    fn coalesce(self, other: &Self) -> Self {
        Self {
            descriptor: self.descriptor.or(other.descriptor.clone()),
            message: self.message.or(other.message.clone()),
        }
    }
}

impl RequestFactory<DefaultGrpcRequest<serde_json::Value, serde_json::value::Serializer>> for GrpcRequest {
    type Error = crate::Error;
    async fn produce(
        &self,
        destination: &http::Uri,
        target: &str,
        template: &Template,
    ) -> Result<DefaultGrpcRequest<serde_json::Value, serde_json::value::Serializer>, Self::Error> {
        let pool = self.descriptor_pool(destination).await?;
        let destination = destination.clone();
        let (service, method) = Self::service_method(&pool, target)?;
        let message = self.message.as_ref().unwrap_or_else(|| todo!()).produce();
        let codec = MethodCodec::new(method.clone()); // TODO remove clone

        Ok(DefaultGrpcRequest { destination, service, method, codec, message })
    }
}
impl GrpcRequest {
    pub fn service_method(pool: &DescriptorPool, target: &str) -> crate::Result<(ServiceDescriptor, MethodDescriptor)> {
        let (svc, mtd) =
            target.split_once('/').ok_or_else(|| GrpcRequestError::FailToParse(target.to_string())).box_err()?; // TODO only one '/' ?
        let service =
            pool.get_service_by_name(svc).ok_or_else(|| GrpcRequestError::NoService(target.to_string())).box_err()?;
        let method = service
            .methods()
            .find(|m| m.name() == mtd)
            .ok_or_else(|| GrpcRequestError::NoMethod(target.to_string()))
            .box_err()?;
        Ok((service, method))
    }
    pub async fn descriptor_pool(&self, destination: &http::Uri) -> crate::Result<DescriptorPool> {
        if let Some(descriptor) = self.descriptor.as_ref() {
            Self::descriptor_from_file(descriptor).await
        } else {
            Self::descriptor_from_reflection(destination).await
        }
    }

    pub async fn descriptor_from_file(path: &PathBuf) -> crate::Result<DescriptorPool> {
        let mut descriptor_bytes = Vec::new();
        File::open(path).box_err()?.read_to_end(&mut descriptor_bytes).box_err()?;
        DescriptorPool::decode(Bytes::from(descriptor_bytes)).box_err()
    }

    pub async fn descriptor_from_reflection(destination: &http::Uri) -> crate::Result<DescriptorPool> {
        // let conn = Channel::builder(destination.clone()).connect().await.unwrap_or_else(|_| todo!());
        // let mut client = ServerReflectionClient::new(conn);
        // let host = destination.host().unwrap_or_else(|| todo!()).to_string();
        // let request_stream = futures::stream::once(async move {
        //     ServerReflectionRequest {
        //         host,
        //         message_request: Some(MessageRequest::FileContainingSymbol("counter.Counter".into())),
        //     }
        // });
        // let mut streaming =
        //     client.server_reflection_info(request_stream).await.unwrap_or_else(|_| todo!()).into_inner();

        // while let Some(recv) = streaming.next().await {
        //     match recv {
        //         Ok(resp) => {
        //             let msg = resp.message_response.unwrap_or_else(|| todo!());
        //             match msg {
        //                 MessageResponse::FileDescriptorResponse(resp) => {
        //                     let c = resp.file_descriptor_proto.concat();
        //                     return DescriptorPool::decode(Bytes::from(c)).box_err();
        //                 }
        //                 _ => todo!(),
        //             }
        //         }
        //         Err(e) => todo!("{}", e),
        //     }
        // }
        // todo!()
        let channel = Channel::builder(destination.clone()).connect().await.unwrap_or_else(|e| todo!("{}", e));
        let mut client = tonic::client::Grpc::new(channel);
        client.ready().await.unwrap_or_else(|e| todo!("{}", e));

        let pool = DescriptorPool::decode(FILE_DESCRIPTOR_SET).unwrap_or_else(|e| todo!("{}", e));
        let mut request = DynamicMessage::new(
            pool.get_message_by_name("grpc.reflection.v1.ServerReflectionRequest").unwrap_or_else(|| unreachable!()),
        );
        request.set_field_by_name(
            "host",
            prost_reflect::Value::String(destination.host().unwrap_or_else(|| todo!()).to_string()),
        );
        request
            .set_field_by_name("file_containing_symbol", prost_reflect::Value::String("counter.Counter".to_string()));

        let response = client
            .unary(
                tonic::Request::new(request),
                PathAndQuery::from_static("/grpc.reflection.v1.ServerReflection/ServerReflectionInfo"),
                dynamic_codec::DynamicReflectionCodec,
            )
            .await
            .unwrap_or_else(|e| todo!("{}", e))
            .into_inner();

        dbg!(&response);
        let mut pool = (*response.get_field_by_name("file_descriptor_proto").unwrap_or_else(|| unreachable!())).clone();
        dbg!(&pool);
        let ret =
            DescriptorPool::decode(pool.as_bytes_mut().unwrap_or_else(|| todo!())).unwrap_or_else(|e| todo!("{}", e));

        Ok(ret)
    }
}

impl GrpcMessage {
    // TODO!!!
    pub fn produce(&self) -> serde_json::Value {
        match self {
            Self::Empty => todo!(),
            Self::Plaintext(_) => todo!(),
            #[cfg(feature = "json")]
            // Self::Json(v) => DynamicMessage::deserialize(descriptor, v).unwrap_or_else(|e| todo!("{}", e)),
            Self::Json(v) => v.clone(),
        }
    }
}

pub mod dynamic_codec {
    use bytes::Buf;
    use prost::Message;
    use prost_reflect::{DescriptorPool, DynamicMessage};
    use tonic::{
        codec::{Codec, Decoder, Encoder},
        Status,
    };
    use tonic_reflection::pb::v1::FILE_DESCRIPTOR_SET;

    #[derive(Debug, Clone, PartialEq, Eq)]
    pub struct DynamicReflectionCodec;

    impl Codec for DynamicReflectionCodec {
        type Encode = DynamicMessage;
        type Decode = DynamicMessage;
        type Encoder = DynamicReflectionEncoder;
        type Decoder = DynamicReflectionDecoder;

        fn encoder(&mut self) -> Self::Encoder {
            DynamicReflectionEncoder
        }

        fn decoder(&mut self) -> Self::Decoder {
            DynamicReflectionDecoder
        }
    }

    #[derive(Debug)]
    pub struct DynamicReflectionEncoder;
    impl Encoder for DynamicReflectionEncoder {
        type Item = DynamicMessage;
        type Error = Status;

        fn encode(&mut self, item: Self::Item, dst: &mut tonic::codec::EncodeBuf<'_>) -> Result<(), Self::Error> {
            item.encode(dst).map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
            Ok(())
        }
    }

    #[derive(Debug)]
    pub struct DynamicReflectionDecoder;
    impl Decoder for DynamicReflectionDecoder {
        type Item = DynamicMessage;
        type Error = Status;

        fn decode(&mut self, src: &mut tonic::codec::DecodeBuf<'_>) -> Result<Option<Self::Item>, Self::Error> {
            if !src.has_remaining() {
                return Ok(None);
            }
            let pool = DescriptorPool::decode(FILE_DESCRIPTOR_SET)
                .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
            let descriptor =
                pool.get_message_by_name("grpc.reflection.v1.FileDescriptorResponse").unwrap_or_else(|| unreachable!());
            let message = DynamicMessage::decode(descriptor, src)
                .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
            Ok(Some(message))
        }
    }
}
