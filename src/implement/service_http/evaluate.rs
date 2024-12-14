use bytes::Bytes;
use http_body::Body;
use http_body_util::{BodyExt, Collected};
use regex::Regex;
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
        evaluator::{Evaluator, RequestResult},
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
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(deny_unknown_fields, rename_all = "kebab-case")]
pub struct PlaintextEvaluate {
    pub regex: Option<EvaluateTo<String>>,
}
impl PlaintextEvaluate {
    pub fn plaintext_acceptable(&self, body: &Destinations<Bytes>, msg: &mut Vec<EvaluateError>) -> bool {
        let _ = msg; // TODO body[d] can be failed
        match &self.regex {
            Some(EvaluateTo::All(regex)) => body.iter().all(|(_, b)| {
                Regex::new(regex).map(|re| re.is_match(String::from_utf8_lossy(b).as_ref())).unwrap_or(false)
            }),
            Some(EvaluateTo::Destinations(dest)) => dest.iter().all(|(d, regex)| {
                Regex::new(regex)
                    .map(|re| re.is_match(String::from_utf8_lossy(body[d].as_ref()).as_ref()))
                    .unwrap_or(false)
            }),
            None => true,
        }
    }
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
        self.acceptable_parts(res, msg).await
    }
}

impl HttpResponse {
    pub async fn acceptable_parts<B: Body>(
        &self,
        res: Destinations<RequestResult<http::Response<B>>>,
        msg: &mut Vec<EvaluateError>,
    ) -> bool
    where
        B::Error: std::error::Error + Sync + Send + 'static,
    {
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
        self.acceptable_status(&s, msg) && self.acceptable_header(&h, msg) && self.acceptable_body(&b, msg)
    }

    pub fn acceptable_status(&self, status: &Destinations<http::StatusCode>, msg: &mut Vec<EvaluateError>) -> bool {
        let acceptable = match &self.status {
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

    pub fn acceptable_header(&self, headers: &Destinations<http::HeaderMap>, msg: &mut Vec<EvaluateError>) -> bool {
        let acceptable = match &self.header {
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

    pub fn acceptable_body(&self, body: &Destinations<Bytes>, msg: &mut Vec<EvaluateError>) -> bool {
        match &self.body {
            BodyEvaluate::AnyOrEqual => Self::assault_or_compare(body, |_| true),
            BodyEvaluate::Plaintext(p) => p.plaintext_acceptable(body, msg),
            #[cfg(feature = "json")]
            BodyEvaluate::Json(e) => e.json_acceptable(body, msg),
        }
    }

    pub fn assault_or_compare<T: PartialEq, F: Fn((&String, &T)) -> bool>(d: &Destinations<T>, f: F) -> bool {
        if d.len() == 1 {
            Self::validate_all(d, f)
        } else {
            Self::compare_all(d)
        }
    }
    pub fn validate_all<T, F: Fn((&String, &T)) -> bool>(d: &Destinations<T>, f: F) -> bool {
        d.iter().all(f)
    }
    pub fn compare_all<T: PartialEq>(status: &Destinations<T>) -> bool {
        let v: Vec<_> = status.values().collect();
        v.windows(2).all(|w| w[0] == w[1])
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
