use bytes::Bytes;
use http_body::Body;
use http_body_util::{BodyExt, Collected};
use serde::{Deserialize, Serialize};

#[cfg(feature = "json")]
use crate::assault::evaluate::json::JsonEvaluate;
use crate::{
    assault::{
        destinations::{AllOr, Destinations},
        evaluate::plaintext::PlaintextEvaluate,
        evaluator::{Acceptable, Evaluator},
        messages::Messages,
        metrics::RequestResult,
    },
    error::EvaluateError,
    interface::helper::{coalesce::Coalesce, http_serde_priv, is_default::IsDefault},
};

#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
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
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "kebab-case")]
pub enum StatusEvaluate {
    #[default]
    OkOrEqual,
    Expect(AllOr<http_serde_priv::StatusCode>),
    Ignore,
}
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "kebab-case")]
pub enum HeaderEvaluate {
    #[default]
    AnyOrEqual,
    Expect(AllOr<http_serde_priv::HeaderMap>),
    Ignore,
}
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
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

impl<B> Evaluator<http::Response<B>> for HttpResponse
where
    B: Body,
    B::Error: std::error::Error + Sync + Send + 'static,
{
    type Message = EvaluateError;
    async fn evaluate(
        &self,
        res: Destinations<RequestResult<http::Response<B>>>,
        msg: &mut Messages<Self::Message>,
    ) -> bool {
        let Some(responses) = msg.response_destinations_with(res, EvaluateError::RequestError) else {
            return false;
        };
        let Some(parts) = msg.push_if_err(HttpResponse::unzip_parts(responses).await) else {
            return false;
        };

        self.accept(&parts, msg)
    }
}
impl Acceptable<(http::StatusCode, http::HeaderMap, Bytes)> for HttpResponse {
    type Message = EvaluateError;
    fn accept(
        &self,
        parts: &Destinations<(http::StatusCode, http::HeaderMap, Bytes)>,
        msg: &mut Messages<Self::Message>,
    ) -> bool {
        let (mut status, mut headers, mut body) = (Destinations::new(), Destinations::new(), Destinations::new());
        for (name, (s, h, b)) in parts {
            status.insert(name.clone(), s);
            headers.insert(name.clone(), h);
            body.insert(name.clone(), b);
        }
        self.status.accept(&status, msg) && self.header.accept(&headers, msg) && self.body.accept(&body, msg)
    }
}
impl HttpResponse {
    pub async fn unzip_parts<B>(
        responses: Destinations<http::Response<B>>,
    ) -> Result<Destinations<(http::StatusCode, http::HeaderMap, Bytes)>, EvaluateError>
    where
        B: Body,
        B::Error: std::error::Error + Sync + Send + 'static,
    {
        let mut parts = Destinations::new();
        for (name, response) in responses {
            let (http::response::Parts { status, headers, .. }, body) = response.into_parts();
            let bytes = BodyExt::collect(body)
                .await
                .map(Collected::to_bytes)
                .map_err(|e| EvaluateError::FailToCollectBody(e.into()))?;
            parts.insert(name, (status, headers, bytes));
        }
        Ok(parts)
    }
}

impl Acceptable<&http::StatusCode> for StatusEvaluate {
    type Message = EvaluateError;
    fn accept(&self, status: &Destinations<&http::StatusCode>, msg: &mut Messages<Self::Message>) -> bool {
        let acceptable = match &self {
            StatusEvaluate::OkOrEqual => Self::assault_or_compare(status, |(_, s)| s.is_success()),
            StatusEvaluate::Expect(AllOr::All(code)) => Self::validate_all(status, |(_, s)| s == &&**code),
            StatusEvaluate::Expect(AllOr::Destinations(code)) => {
                // TODO subset ?
                status == &code.iter().map(|(d, c)| (d.to_string(), &**c)).collect()
            }
            StatusEvaluate::Ignore => true,
        };
        if !acceptable {
            msg.push_err(EvaluateError::UnacceptableStatus);
        }
        acceptable
    }
}

impl Acceptable<&http::HeaderMap> for HeaderEvaluate {
    type Message = EvaluateError;
    fn accept(&self, headers: &Destinations<&http::HeaderMap>, msg: &mut Messages<Self::Message>) -> bool {
        let acceptable = match &self {
            HeaderEvaluate::AnyOrEqual => Self::assault_or_compare(headers, |_| true),
            HeaderEvaluate::Expect(AllOr::All(header)) => Self::validate_all(headers, |(_, h)| h == &&**header),
            HeaderEvaluate::Expect(AllOr::Destinations(header)) => {
                // TODO subset ?
                headers == &header.iter().map(|(d, h)| (d.to_string(), &**h)).collect()
            }
            HeaderEvaluate::Ignore => true,
        };
        if !acceptable {
            msg.push_err(EvaluateError::UnacceptableHeaderMap);
        }
        acceptable
    }
}

impl Acceptable<&Bytes> for BodyEvaluate {
    type Message = EvaluateError;
    fn accept(&self, body: &Destinations<&Bytes>, msg: &mut Messages<Self::Message>) -> bool {
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
    use std::time::SystemTime;

    use crate::assault::measure::metrics::MeasuredResponse;

    use super::*;

    #[tokio::test]
    async fn test_default_assault_evaluate() {
        let evaluator = HttpResponse::default();

        let ok =
            http::Response::builder().status(http::StatusCode::OK).body(http_body_util::Empty::<Bytes>::new()).unwrap();
        let responses = Destinations::from_iter(vec![(
            "test".to_string(),
            Ok(MeasuredResponse::new(ok, SystemTime::now(), Default::default())),
        )]);
        let mut msg = Messages::new();
        let result = evaluator.evaluate(responses, &mut msg).await;
        assert!(result);
        assert!(msg.is_empty());

        let unavailable = http::Response::builder()
            .status(http::StatusCode::SERVICE_UNAVAILABLE)
            .body(http_body_util::Empty::<Bytes>::new())
            .unwrap();
        let responses = Destinations::from_iter(vec![(
            "test".to_string(),
            Ok(MeasuredResponse::new(unavailable, SystemTime::now(), Default::default())),
        )]);
        let mut msg = Messages::new();
        let result = evaluator.evaluate(responses, &mut msg).await;
        assert!(!result);
        assert!(matches!(msg.as_slice(), [EvaluateError::UnacceptableStatus]));
    }

    // TODO more tests
}
