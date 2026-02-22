use std::{convert::Infallible, fmt::Debug};

use bytes::Bytes;
use http_body::Body;
use relentless::{http_newtype_serde, shot::contract::RequestSource};
use semigroup::Semigroup;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize, Semigroup)]
#[serde(deny_unknown_fields, rename_all = "kebab-case")]
#[semigroup(with = "semigroup::op::Coalesce")]
pub struct HttpRequest {
    #[cfg_attr(feature = "yaml", serde(with = "serde_yaml::with::singleton_map_recursive"))]
    #[serde(default)]
    pub method: Option<http_newtype_serde::Method>,
    #[cfg_attr(feature = "yaml", serde(with = "serde_yaml::with::singleton_map_recursive"))]
    #[serde(default)]
    pub headers: Option<http_newtype_serde::HeaderMap>,
    #[cfg_attr(feature = "yaml", serde(with = "serde_yaml::with::singleton_map_recursive"))]
    #[serde(default)]
    pub body: Option<HttpRequestBody>,
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

impl<ReqB: Body + Default + From<Bytes> + Debug> RequestSource<http::Request<ReqB>> for HttpRequest {
    type Error = Infallible;

    #[tracing::instrument(ret, err)]
    async fn produce(&self, destination: &http::Uri, target: &str) -> Result<http::Request<ReqB>, Self::Error> {
        let uri =
            http::uri::Builder::from(destination.clone()).path_and_query(target).build().unwrap_or_else(|_| todo!());
        let method = self.method.as_deref().unwrap_or(&Default::default()).clone();
        let header = self.headers.as_deref().unwrap_or(&Default::default()).clone();
        let body = self.body.as_ref().unwrap_or(&Default::default()).produce(destination, target).await?;
        let mut request = http::Request::builder().uri(uri).method(method).body(body).unwrap_or_else(|_| todo!());
        request.headers_mut().extend(header);
        Ok(request)
    }
}

impl<ReqB: Body + Default + From<Bytes>> RequestSource<ReqB> for HttpRequestBody {
    type Error = Infallible;

    async fn produce(&self, _: &http::Uri, _: &str) -> Result<ReqB, Self::Error> {
        match self {
            Self::Empty => Ok(Default::default()),
            Self::Plaintext(s) => Ok(Bytes::from(s.to_string()).into()),
            #[cfg(feature = "json")]
            Self::Json(v) => {
                let s = serde_json::to_string(v).unwrap_or_else(|_| todo!());
                Ok(Bytes::from(s).into())
            }
        }
    }
}
