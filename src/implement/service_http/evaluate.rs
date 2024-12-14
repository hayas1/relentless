use bytes::Bytes;
use http_body::Body;
use http_body_util::{BodyExt, Collected};
use serde::{Deserialize, Serialize};

#[cfg(feature = "json")]
use crate::service::evaluate::json::JsonEvaluate;
use crate::{
    error::EvaluateError,
    interface::{
        config::EvaluateTo,
        helper::{coalesce::Coalesce, http_serde_priv, is_default::IsDefault},
    },
    service::{
        destinations::Destinations,
        evaluate::plaintext::PlaintextEvaluate,
        evaluator::{Acceptable, Evaluator, RequestResult},
    },
};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(deny_unknown_fields, rename_all = "kebab-case")]
pub struct HttpResponse {
    #[serde(default, skip_serializing_if = "IsDefault::is_default")]
    #[cfg_attr(feature = "yaml", serde(with = "serde_yaml::with::singleton_map_recursive"))]
    pub status: StatusEvaluate,
    #[serde(default, skip_serializing_if = "IsDefault::is_default")]
    #[cfg_attr(feature = "yaml", serde(with = "serde_yaml::with::singleton_map_recursive"))]
    pub header: HeaderEvaluate,
    #[serde(default, skip_serializing_if = "IsDefault::is_default")]
    #[cfg_attr(feature = "yaml", serde(with = "serde_yaml::with::singleton_map_recursive"))]
    pub body: BodyEvaluate,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(deny_unknown_fields, rename_all = "kebab-case")]
pub enum StatusEvaluate {
    #[default]
    OkOrEqual,
    Expect(EvaluateTo<http_serde_priv::StatusCode>),
    Ignore,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(deny_unknown_fields, rename_all = "kebab-case")]
pub enum HeaderEvaluate {
    #[default]
    AnyOrEqual,
    Expect(EvaluateTo<http_serde_priv::HeaderMap>),
    Ignore,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(deny_unknown_fields, rename_all = "kebab-case")]
pub enum BodyEvaluate {
    #[default]
    AnyOrEqual,
    Plaintext(PlaintextEvaluate),
    #[cfg(feature = "json")]
    Json(JsonEvaluate),
}

impl Coalesce for HttpResponse {
    fn coalesce(self, other: &Self) -> Self {
        Self {
            status: self.status.coalesce(&other.status),
            header: self.header.coalesce(&other.header),
            body: self.body.coalesce(&other.body),
        }
    }
}
impl Coalesce for StatusEvaluate {
    fn coalesce(self, other: &Self) -> Self {
        if self.is_default() {
            other.clone()
        } else {
            self
        }
    }
}
impl Coalesce for HeaderEvaluate {
    fn coalesce(self, other: &Self) -> Self {
        if self.is_default() {
            other.clone()
        } else {
            self
        }
    }
}
impl Coalesce for BodyEvaluate {
    fn coalesce(self, other: &Self) -> Self {
        if self.is_default() {
            other.clone()
        } else {
            self
        }
    }
}

impl<B: Body> Evaluator<http::Response<B>> for HttpResponse
where
    B::Error: std::error::Error + Sync + Send + 'static,
{
    type Message = EvaluateError;
    async fn evaluate(
        &self,
        res: Destinations<RequestResult<http::Response<B>>>,
        msg: &mut Vec<Self::Message>,
    ) -> bool {
        // TODO `Unzip` trait ?
        let (mut s, mut h, mut b) = (Destinations::new(), Destinations::new(), Destinations::new());
        for (name, r) in res {
            let response = match r {
                RequestResult::Response(r) => r,
                RequestResult::Timeout(d) => {
                    msg.push(EvaluateError::RequestTimeout(d));
                    return false;
                }
            };
            let (http::response::Parts { status, headers, .. }, body) = response.into_parts();
            let bytes = match BodyExt::collect(body).await.map(Collected::to_bytes) {
                Ok(b) => b,
                Err(e) => {
                    msg.push(EvaluateError::FailToCollectBody(e.into()));
                    return false;
                }
            };
            s.insert(name.clone(), status);
            h.insert(name.clone(), headers);
            b.insert(name.clone(), bytes);
        }
        self.status.accept(&s, msg) && self.header.accept(&h, msg) && self.body.accept(&b, msg)
    }
}

impl Acceptable<http::StatusCode> for StatusEvaluate {
    type Message = EvaluateError;
    fn accept(&self, status: &Destinations<http::StatusCode>, msg: &mut Vec<Self::Message>) -> bool {
        let acceptable = match &self {
            StatusEvaluate::OkOrEqual => Self::assault_or_compare(status, |(_, s)| s.is_success()),
            StatusEvaluate::Expect(EvaluateTo::All(code)) => Self::validate_all(status, |(_, s)| s == &**code),
            StatusEvaluate::Expect(EvaluateTo::Destinations(code)) => {
                // TODO subset ?
                status == &code.iter().map(|(d, c)| (d.to_string(), **c)).collect()
            }
            StatusEvaluate::Ignore => true,
        };
        if !acceptable {
            msg.push(EvaluateError::UnacceptableStatus);
        }
        acceptable
    }
}

impl Acceptable<http::HeaderMap> for HeaderEvaluate {
    type Message = EvaluateError;
    fn accept(&self, headers: &Destinations<http::HeaderMap>, msg: &mut Vec<Self::Message>) -> bool {
        let acceptable = match &self {
            HeaderEvaluate::AnyOrEqual => Self::assault_or_compare(headers, |_| true),
            HeaderEvaluate::Expect(EvaluateTo::All(header)) => Self::validate_all(headers, |(_, h)| h == &**header),
            HeaderEvaluate::Expect(EvaluateTo::Destinations(header)) => {
                // TODO subset ?
                headers == &header.iter().map(|(d, h)| (d.to_string(), (**h).clone())).collect()
            }
            HeaderEvaluate::Ignore => true,
        };
        if !acceptable {
            msg.push(EvaluateError::UnacceptableHeaderMap);
        }
        acceptable
    }
}

impl Acceptable<Bytes> for BodyEvaluate {
    type Message = EvaluateError;
    fn accept(&self, body: &Destinations<Bytes>, msg: &mut Vec<Self::Message>) -> bool {
        match &self {
            BodyEvaluate::AnyOrEqual => Self::assault_or_compare(body, |_| true),
            BodyEvaluate::Plaintext(p) => p.accept(body, msg),
            #[cfg(feature = "json")]
            BodyEvaluate::Json(e) => e.accept(body, msg),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_default_assault_evaluate() {
        let evaluator = HttpResponse::default();

        let ok =
            http::Response::builder().status(http::StatusCode::OK).body(http_body_util::Empty::<Bytes>::new()).unwrap();
        let responses = Destinations::from_iter(vec![("test".to_string(), RequestResult::Response(ok))]);
        let mut msg = Vec::new();
        let result = evaluator.evaluate(responses, &mut msg).await;
        assert!(result);
        assert!(msg.is_empty());

        let unavailable = http::Response::builder()
            .status(http::StatusCode::SERVICE_UNAVAILABLE)
            .body(http_body_util::Empty::<Bytes>::new())
            .unwrap();
        let responses = Destinations::from_iter(vec![("test".to_string(), RequestResult::Response(unavailable))]);
        let mut msg = Vec::new();
        let result = evaluator.evaluate(responses, &mut msg).await;
        assert!(!result);
        assert!(matches!(msg[0], EvaluateError::UnacceptableStatus));
    }

    // TODO more tests
}
