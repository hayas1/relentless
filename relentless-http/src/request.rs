use std::{convert::Infallible, fmt::Debug};

use bytes::Bytes;
use http_body::Body;
use relentless::{http_newtype_serde, shot::contract::RequestSource, template::Template};
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
    Json(serde_json::Value),
}

impl HttpRequestBody {
    fn render(&self, template: &Template) -> relentless::Result<Self> {
        match self {
            Self::Empty => Ok(Self::Empty),
            Self::Plaintext(s) => Ok(Self::Plaintext(template.render(s)?)),
            Self::Json(v) => Ok(Self::Json(template.render_json_recursive(v)?)),
        }
    }

    async fn produce_bytes<ReqB: Body + Default + From<Bytes>>(&self) -> ReqB {
        match self {
            Self::Empty => Default::default(),
            Self::Plaintext(s) => Bytes::from(s.to_string()).into(),
            Self::Json(v) => {
                let s = serde_json::to_string(v).unwrap_or_else(|_| todo!());
                Bytes::from(s).into()
            }
        }
    }
}

impl<ReqB: Body + Default + From<Bytes> + Debug> RequestSource<http::Request<ReqB>> for HttpRequest {
    type Error = relentless::Error;

    #[tracing::instrument(ret, err)]
    async fn produce(
        &self,
        destination: &http::Uri,
        target: &str,
        template: &Template,
    ) -> Result<http::Request<ReqB>, Self::Error> {
        let target = template.render(target)?;
        let uri = http::uri::Builder::from(destination.clone())
            .path_and_query(target.as_str())
            .build()
            .unwrap_or_else(|_| todo!());
        let method = self.method.as_deref().unwrap_or(&Default::default()).clone();
        let raw_headers = self.headers.as_deref().unwrap_or(&Default::default()).clone();
        let mut header = http::HeaderMap::new();
        for (k, v) in raw_headers.iter() {
            let rendered = template.render(v.to_str().unwrap_or_default())?;
            let new_v = http::HeaderValue::from_str(&rendered).unwrap_or_else(|_| todo!());
            header.insert(k, new_v);
        }
        let rendered_body = self.body.as_ref().unwrap_or(&Default::default()).render(template)?;
        let body = rendered_body.produce_bytes::<ReqB>().await;
        let mut request = http::Request::builder().uri(uri).method(method).body(body).unwrap_or_else(|_| todo!());
        request.headers_mut().extend(header);
        Ok(request)
    }
}

impl<ReqB: Body + Default + From<Bytes>> RequestSource<ReqB> for HttpRequestBody {
    type Error = Infallible;

    async fn produce(&self, _: &http::Uri, _: &str, _template: &Template) -> Result<ReqB, Self::Error> {
        Ok(self.produce_bytes().await)
    }
}
