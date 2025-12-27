use std::{
    collections::HashMap,
    fmt::{Debug, Display},
};

use bytes::Bytes;
use futures::{StreamExt, TryStreamExt};
use http::{HeaderMap, StatusCode};
use http_body::Body;
use http_body_util::BodyExt;
#[cfg(feature = "json")]
use relentless::evaluator::json::JsonEvaluator;
use relentless::{
    error::EvaluateError,
    evaluator::{evaluate::Evaluator, expect::ExpectEvaluator, plaintext::PlaintextEvaluator},
    http_newtype_serde,
    shot::{contract::ResponseSink, destinations::Destinations},
};
use semigroup::Semigroup;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize, Semigroup)]
#[serde(deny_unknown_fields, rename_all = "kebab-case")]
#[semigroup(with = "semigroup::op::Coalesce")]
pub struct HttpResponse {
    #[cfg_attr(feature = "yaml", serde(with = "serde_yaml::with::singleton_map_recursive"))]
    #[serde(default)]
    pub status: Option<HttpResponseStatus>,
    #[cfg_attr(feature = "yaml", serde(with = "serde_yaml::with::singleton_map_recursive"))]
    #[serde(default)]
    pub header: Option<HttpResponseHeaders>,
    #[cfg_attr(feature = "yaml", serde(with = "serde_yaml::with::singleton_map_recursive"))]
    #[serde(default)]
    pub body: Option<HttpResponseBody>,
}
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "kebab-case")]
pub enum HttpResponseStatus {
    #[default]
    OkOrEqual,
    Expect(ExpectEvaluator<http_newtype_serde::StatusCode>),
    Ignore,
}
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "kebab-case")]
pub enum HttpResponseHeaders {
    #[default]
    AnyOrEqual,
    // Allowlist(Vec<String>), // TODO
    Expect(ExpectEvaluator<http_newtype_serde::HeaderMap>),
    Ignore,
}
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "kebab-case")]
pub enum HttpResponseBody {
    #[default]
    AnyOrEqual,
    Plaintext(PlaintextEvaluator),
    #[cfg(feature = "json")]
    Json(JsonEvaluator),
}

impl<ResB, E> ResponseSink<Result<http::Response<ResB>, E>> for HttpResponse
where
    ResB: Body + Debug + Send,
    ResB::Data: Send,
    E: Display + Debug + Send,
{
    type Error = EvaluateError;
    #[tracing::instrument(err)]
    async fn consume(&self, res: Destinations<Result<http::Response<ResB>, E>>) -> Result<(), Self::Error> {
        let buffers = res.len().max(1);
        let collected: Destinations<_> = futures::stream::iter(res)
            .map(|(d, r)| async {
                let (parts, body) = r.map_err(EvaluateError::custom)?.into_parts();
                let collected = body.collect().await.map_err(|_| EvaluateError::custom("failed to collect body"))?;
                Ok((d, http::Response::from_parts(parts, collected.to_bytes())))
            })
            .buffer_unordered(buffers)
            .try_collect()
            .await?;
        if collected.len() == 1 {
            self.evaluate_shots(collected)
        } else {
            self.evaluate_compares(collected)
        }
    }
}
impl Evaluator<http::Response<Bytes>> for HttpResponse {
    type Error = EvaluateError;
    fn evaluate_shot(&self, res: &http::Response<Bytes>) -> Result<(), Self::Error> {
        self.status.as_ref().unwrap_or(&Default::default()).evaluate_shot(&res.status())?;
        self.header.as_ref().unwrap_or(&Default::default()).evaluate_shot(res.headers())?;
        self.body.as_ref().unwrap_or(&Default::default()).evaluate_shot(res.body())?;
        Ok(())
    }
    fn evaluate_compare(&self, res1: &http::Response<Bytes>, res2: &http::Response<Bytes>) -> Result<(), Self::Error> {
        self.status.as_ref().unwrap_or(&Default::default()).evaluate_compare(&res1.status(), &res2.status())?;
        self.header.as_ref().unwrap_or(&Default::default()).evaluate_compare(res1.headers(), res2.headers())?;
        self.body.as_ref().unwrap_or(&Default::default()).evaluate_compare(res1.body(), res2.body())?;
        Ok(())
    }
}

impl Evaluator<StatusCode> for HttpResponseStatus {
    type Error = EvaluateError;
    fn evaluate_shot(&self, res: &StatusCode) -> Result<(), Self::Error> {
        match self {
            Self::OkOrEqual => self.evaluate(res.is_success(), |_| EvaluateError::custom("not success status")),
            Self::Expect(e) => todo!(),
            Self::Ignore => Ok(()),
        }
    }
    fn evaluate_compare(&self, res1: &StatusCode, res2: &StatusCode) -> Result<(), Self::Error> {
        match self {
            Self::OkOrEqual => self.evaluate(res1 == res2, |_| EvaluateError::custom("not equal status")),
            Self::Expect(e) => todo!(),
            Self::Ignore => Ok(()),
        }
    }
}
impl Evaluator<HeaderMap> for HttpResponseHeaders {
    type Error = EvaluateError;
    fn evaluate_shot(&self, res: &HeaderMap) -> Result<(), Self::Error> {
        match self {
            Self::AnyOrEqual => Ok(()),
            Self::Expect(e) => todo!(),
            Self::Ignore => Ok(()),
        }
    }
    fn evaluate_compare(&self, res1: &HeaderMap, res2: &HeaderMap) -> Result<(), Self::Error> {
        let (resp1, resp2): (HashMap<_, _>, HashMap<_, _>) = (
            self.allowlist().iter().filter_map(|&k| Some((k, res1.get(k)?))).collect(),
            self.allowlist().iter().filter_map(|&k| Some((k, res2.get(k)?))).collect(),
        );
        match self {
            Self::AnyOrEqual => self.evaluate(resp1 == resp2, |_| EvaluateError::custom("not equal headers")),
            Self::Expect(e) => todo!(),
            Self::Ignore => Ok(()),
        }
    }
}
impl HttpResponseHeaders {
    pub const DEFAULT_ALLOWLIST: &[&str] =
        &["content-type", "content-length", "content-language", "content-encoding", "cache-control"];
    pub fn allowlist(&self) -> &[&str] {
        Self::DEFAULT_ALLOWLIST
    }
}
impl Evaluator<Bytes> for HttpResponseBody {
    type Error = EvaluateError;
    fn evaluate_shot(&self, res: &Bytes) -> Result<(), Self::Error> {
        match self {
            Self::AnyOrEqual => Ok(()),
            Self::Plaintext(e) => todo!(),
            #[cfg(feature = "json")]
            Self::Json(e) => todo!(),
        }
    }
    fn evaluate_compare(&self, res1: &Bytes, res2: &Bytes) -> Result<(), Self::Error> {
        match self {
            Self::AnyOrEqual => self.evaluate(res1 == res2, |_| EvaluateError::custom("not equal body")),
            Self::Plaintext(e) => todo!(),
            #[cfg(feature = "json")]
            Self::Json(e) => todo!(),
        }
    }
}
