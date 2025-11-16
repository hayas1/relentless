use http_body::Body;
#[cfg(feature = "json")]
use relentless::evaluator::json::JsonEvaluator;
use relentless::{
    evaluator::{expect::ExpectEvaluator, plaintext::PlaintextEvaluator, Evaluator},
    http_newtype_serde,
    shot::destinations::Destinations,
};
use serde::{Deserialize, Serialize};

use crate::client::HttpClient;

#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "kebab-case")]
pub struct HttpResponse {
    #[serde(default)]
    #[cfg_attr(feature = "yaml", serde(with = "serde_yaml::with::singleton_map_recursive"))]
    pub status: HttpRequestStatus,
    #[serde(default)]
    #[cfg_attr(feature = "yaml", serde(with = "serde_yaml::with::singleton_map_recursive"))]
    pub header: HttpRequestHeaders,
    #[serde(default)]
    #[cfg_attr(feature = "yaml", serde(with = "serde_yaml::with::singleton_map_recursive"))]
    pub body: HttpRequestBody,
}
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "kebab-case")]
pub enum HttpRequestStatus {
    #[default]
    OkOrEqual,
    Expect(ExpectEvaluator<http_newtype_serde::StatusCode>),
    Ignore,
}
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "kebab-case")]
pub enum HttpRequestHeaders {
    #[default]
    AnyOrEqual,
    Expect(ExpectEvaluator<http_newtype_serde::HeaderMap>),
    Ignore,
}
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "kebab-case")]
pub enum HttpRequestBody {
    #[default]
    AnyOrEqual,
    Plaintext(PlaintextEvaluator),
    #[cfg(feature = "json")]
    Json(JsonEvaluator),
}

impl<ReqB: Send, ResB: Body + Send> Evaluator<HttpClient<ReqB, ResB>> for HttpResponse {
    type Response = http::Response<ResB>;

    async fn evaluate(&self, res: Destinations<Self::Response>) -> bool {
        todo!()
    }
}
