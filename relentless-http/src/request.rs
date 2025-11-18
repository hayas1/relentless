use std::marker::PhantomData;

use bytes::Bytes;
use http_body::Body;
use relentless::{generator::Generator, http_newtype_serde};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "kebab-case")]
pub struct HttpRequest<B> {
    #[serde(default)]
    #[cfg_attr(feature = "yaml", serde(with = "serde_yaml::with::singleton_map_recursive"))]
    pub method: Option<http_newtype_serde::Method>,
    #[serde(default)]
    #[cfg_attr(feature = "yaml", serde(with = "serde_yaml::with::singleton_map_recursive"))]
    pub headers: Option<http_newtype_serde::HeaderMap>,
    #[serde(default)]
    #[cfg_attr(feature = "yaml", serde(with = "serde_yaml::with::singleton_map_recursive"))]
    pub body: HttpRequestBody<B>,
}
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "kebab-case")]
pub enum HttpRequestBody<B> {
    #[default]
    Empty,
    Plaintext(String),
    #[cfg(feature = "json")]
    Json(serde_json::Value),
    Phantom(PhantomData<B>),
}

impl<ReqB: Body + Send + Sync> Generator for HttpRequest<ReqB> {
    type Request = http::Request<ReqB>;
    type Error = reqwest::Error;

    async fn generate(&self, _destination: &http::Uri, _target: &str) -> Result<Self::Request, Self::Error> {
        todo!()
    }
}

impl<ReqB: Body + From<Bytes> + Default + Send + Sync> Generator for HttpRequestBody<ReqB> {
    type Request = ReqB;
    type Error = reqwest::Error;

    async fn generate(&self, _: &http::Uri, _: &str) -> Result<Self::Request, Self::Error> {
        match self {
            Self::Empty => Ok(Default::default()),
            Self::Plaintext(s) => Ok(Bytes::from(s.to_string()).into()),
            #[cfg(feature = "json")]
            Self::Json(v) => {
                todo!()
            }
            Self::Phantom(_) => unreachable!(),
        }
    }
}
