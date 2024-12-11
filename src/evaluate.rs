use std::time::Duration;

use bytes::Bytes;
use http_body::Body;
use http_body_util::{BodyExt, Collected};
use regex::Regex;
#[cfg(feature = "json")]
use serde_json::Value;

#[cfg(feature = "json")]
use crate::config::JsonEvaluate;
use crate::config::{EvaluateTo, HttpEvaluate, Severity};
use crate::error::EvaluateError;
use crate::{
    config::{destinations::Destinations, BodyEvaluate, HeaderEvaluate, StatusEvaluate},
    error::WrappedResult,
};

pub enum RequestResult<Res> {
    Response(Res),
    Timeout(Duration),
}

#[allow(async_fn_in_trait)] // TODO #[warn(async_fn_in_trait)] by default
pub trait Evaluator<Res> {
    type Message;
    async fn evaluate(
        &self,
        cfg: &HttpEvaluate,
        res: Destinations<RequestResult<Res>>,
        msg: &mut Vec<Self::Message>,
    ) -> bool;
}
pub struct DefaultEvaluator;
impl<B: Body> Evaluator<http::Response<B>> for DefaultEvaluator
where
    B::Error: std::error::Error + Sync + Send + 'static,
{
    type Message = EvaluateError;
    async fn evaluate(
        &self,
        cfg: &HttpEvaluate,
        res: Destinations<RequestResult<http::Response<B>>>,
        msg: &mut Vec<Self::Message>,
    ) -> bool {
        Self::acceptable_parts(cfg, res, msg).await
    }
}

impl DefaultEvaluator {
    pub async fn acceptable_parts<B: Body>(
        cfg: &HttpEvaluate,
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
        Self::acceptable_status(&cfg.status, &s, msg)
            && Self::acceptable_header(&cfg.header, &h, msg)
            && Self::acceptable_body(&cfg.body, &b, msg)
    }

    pub fn acceptable_status(
        cfg: &StatusEvaluate,
        status: &Destinations<http::StatusCode>,
        msg: &mut Vec<EvaluateError>,
    ) -> bool {
        let acceptable = match cfg {
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

    pub fn acceptable_header(
        cfg: &HeaderEvaluate,
        headers: &Destinations<http::HeaderMap>,
        msg: &mut Vec<EvaluateError>,
    ) -> bool {
        let acceptable = match cfg {
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

    pub fn acceptable_body(cfg: &BodyEvaluate, body: &Destinations<Bytes>, msg: &mut Vec<EvaluateError>) -> bool {
        match cfg {
            BodyEvaluate::AnyOrEqual => Self::assault_or_compare(body, |_| true),
            BodyEvaluate::Plaintext(EvaluateTo::All(p)) => Self::validate_all(body, |(_, b)| match &p.regex {
                Some(regex) => {
                    Regex::new(regex).map(|re| re.is_match(String::from_utf8_lossy(b).as_ref())).unwrap_or(false)
                }
                None => true,
            }),
            BodyEvaluate::Plaintext(EvaluateTo::Destinations(dest)) => {
                Self::validate_all(dest, |(d, p)| match &p.regex {
                    Some(regex) => Regex::new(regex)
                        .map(|re| re.is_match(String::from_utf8_lossy(body[d].as_ref()).as_ref()))
                        .unwrap_or(false),
                    None => true,
                })
            }
            #[cfg(feature = "json")]
            BodyEvaluate::Json(e) => Self::json_acceptable(e, body, msg),
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

#[cfg(feature = "json")]
impl DefaultEvaluator {
    pub fn json_acceptable(cfg: &JsonEvaluate, parts: &Destinations<Bytes>, msg: &mut Vec<EvaluateError>) -> bool {
        let values: Vec<_> = match Self::patched(cfg, parts) {
            Ok(values) => values,
            Err(e) => {
                msg.push(EvaluateError::FailToPatchJson(e));
                return false;
            }
        }
        .into_values()
        .collect();

        values.windows(2).all(|w| Self::json_compare(cfg, (&w[0], &w[1]), msg))
    }

    pub fn patched(cfg: &JsonEvaluate, parts: &Destinations<Bytes>) -> WrappedResult<Destinations<Value>> {
        parts
            .iter()
            .map(|(name, body)| {
                let mut value = serde_json::from_slice(body)?;
                if let Err(e) = Self::patch(cfg, name, &mut value) {
                    if cfg.patch_fail.is_none() && parts.len() == 1 || cfg.patch_fail > Some(Severity::Warn) {
                        Err(e)?;
                    }
                }
                Ok((name.clone(), value))
            })
            .collect()
    }
    pub fn patch(cfg: &JsonEvaluate, name: &str, value: &mut Value) -> Result<(), json_patch::PatchError> {
        let default_patch = json_patch::Patch::default();
        let patch = match &cfg.patch {
            Some(EvaluateTo::All(p)) => p,
            Some(EvaluateTo::Destinations(patch)) => patch.get(name).unwrap_or(&default_patch),
            None => &default_patch,
        };
        json_patch::patch_unsafe(value, patch)
    }

    pub fn json_compare(cfg: &JsonEvaluate, (va, vb): (&Value, &Value), msg: &mut Vec<EvaluateError>) -> bool {
        let diff = json_patch::diff(va, vb);
        let pointers = Self::pointers(&diff);
        diff.iter().zip(pointers).filter(|(_op, path)| !cfg.ignore.contains(path)).fold(true, |_acc, (_op, path)| {
            msg.push(EvaluateError::Diff(path));
            false
        })
    }

    pub fn pointers(p: &json_patch::Patch) -> Vec<String> {
        // TODO implemented in json_patch ?
        p.iter()
            .map(|op| match op {
                json_patch::PatchOperation::Add(json_patch::AddOperation { path, .. }) => path,
                json_patch::PatchOperation::Remove(json_patch::RemoveOperation { path, .. }) => path,
                json_patch::PatchOperation::Replace(json_patch::ReplaceOperation { path, .. }) => path,
                json_patch::PatchOperation::Move(json_patch::MoveOperation { path, .. }) => path,
                json_patch::PatchOperation::Copy(json_patch::CopyOperation { path, .. }) => path,
                json_patch::PatchOperation::Test(json_patch::TestOperation { path, .. }) => path,
            })
            .map(ToString::to_string)
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_default_assault_evaluate() {
        let evaluator = DefaultEvaluator;

        let ok =
            http::Response::builder().status(http::StatusCode::OK).body(http_body_util::Empty::<Bytes>::new()).unwrap();
        let responses = Destinations::from_iter(vec![("test".to_string(), RequestResult::Response(ok))]);
        let mut msg = Vec::new();
        let result = evaluator.evaluate(&Default::default(), responses, &mut msg).await;
        assert!(result);
        assert!(msg.is_empty());

        let unavailable = http::Response::builder()
            .status(http::StatusCode::SERVICE_UNAVAILABLE)
            .body(http_body_util::Empty::<Bytes>::new())
            .unwrap();
        let responses = Destinations::from_iter(vec![("test".to_string(), RequestResult::Response(unavailable))]);
        let mut msg = Vec::new();
        let result = evaluator.evaluate(&Default::default(), responses, &mut msg).await;
        assert!(!result);
        assert!(matches!(msg[0], EvaluateError::UnacceptableStatus));
    }

    // TODO more tests
}
