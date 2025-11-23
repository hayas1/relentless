use std::convert::Infallible;

use bytes::Bytes;
use http_body::Body;
use relentless::{http_newtype_serde, shot::contract::RequestSource};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "kebab-case")]
pub struct HttpRequest {
    #[serde(default)]
    #[cfg_attr(feature = "yaml", serde(with = "serde_yaml::with::singleton_map_recursive"))]
    pub method: Option<http_newtype_serde::Method>,
    #[serde(default)]
    #[cfg_attr(feature = "yaml", serde(with = "serde_yaml::with::singleton_map_recursive"))]
    pub headers: Option<http_newtype_serde::HeaderMap>,
    #[serde(default)]
    #[cfg_attr(feature = "yaml", serde(with = "serde_yaml::with::singleton_map_recursive"))]
    pub body: HttpRequestBody,
}
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "kebab-case")]
pub enum HttpRequestBody {
    #[default]
    Empty,
    Plaintext(String),
    #[cfg(feature = "json")]
    Json(serde_json::Value),
}

impl<ReqB: Body + From<Bytes> + Default + Send + Sync> RequestSource<http::Request<ReqB>> for HttpRequest {
    type Error = Infallible;

    async fn produce(&self, destination: &http::Uri, target: &str) -> Result<http::Request<ReqB>, Self::Error> {
        let b = self.body.produce(destination, target).await?;
        let request =
            http::Request::builder().uri(destination).method(http::Method::GET).body(b).unwrap_or_else(|_| todo!());
        Ok(request)
    }
}

impl<ReqB: Body + From<Bytes> + Default + Send + Sync> RequestSource<ReqB> for HttpRequestBody {
    type Error = Infallible;

    async fn produce(&self, destination: &http::Uri, target: &str) -> Result<ReqB, Self::Error> {
        match self {
            Self::Empty => Ok(Default::default()),
            Self::Plaintext(s) => Ok(Bytes::from(s.to_string()).into()),
            #[cfg(feature = "json")]
            Self::Json(v) => {
                todo!()
            }
        }
    }
}
