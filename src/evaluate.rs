use bytes::Bytes;
use http_body::Body;
use http_body_util::BodyExt;
#[cfg(feature = "json")]
use serde_json::Value;

#[cfg(feature = "json")]
use crate::config::{JsonEvaluate, PatchTo};
use crate::{
    config::{BodyEvaluate, Destinations, HeaderEvaluate, Protocol, StatusEvaluate},
    error::{Wrap, WrappedResult},
};

#[allow(async_fn_in_trait)] // TODO #[warn(async_fn_in_trait)] by default
pub trait Evaluator<Res> {
    type Error;
    type Message;
    async fn evaluate(
        &self,
        cfg: Option<&Protocol>,
        res: Destinations<Res>,
        msg: &mut Vec<Self::Message>,
    ) -> Result<bool, Self::Error>;
}
pub struct DefaultEvaluator;
impl<ResB: Body> Evaluator<http::Response<ResB>> for DefaultEvaluator
where
    ResB::Error: std::error::Error + Sync + Send + 'static,
{
    type Error = crate::Error;
    type Message = String;
    async fn evaluate(
        &self,
        cfg: Option<&Protocol>,
        res: Destinations<http::Response<ResB>>,
        msg: &mut Vec<Self::Message>,
    ) -> Result<bool, Self::Error> {
        Self::parts(cfg, res, msg).await
    }
}

impl DefaultEvaluator {
    pub async fn parts<ResB: Body>(
        cfg: Option<&Protocol>,
        res: Destinations<http::Response<ResB>>,
        msg: &mut Vec<String>,
    ) -> Result<bool, <Self as Evaluator<http::Response<ResB>>>::Error>
    where
        ResB::Error: std::error::Error + Sync + Send + 'static,
    {
        let (mut s, mut h, mut b) = (Destinations::new(), Destinations::new(), Destinations::new());
        for (name, r) in res {
            let (http::response::Parts { status, headers, .. }, body) = r.into_parts();
            let bytes = BodyExt::collect(body).await.map(|buf| buf.to_bytes()).map_err(Wrap::wrapping)?;
            s.insert(name.clone(), status);
            h.insert(name.clone(), headers);
            b.insert(name.clone(), bytes);
        }
        let evaluate = match &cfg {
            Some(Protocol::Http(http)) => &http.evaluate,
            None => &Default::default(),
        };
        Ok(Self::acceptable_status(&evaluate.status, &s, msg)?
            && Self::acceptable_header(&evaluate.header, &h, msg)?
            && Self::acceptable_body(&evaluate.body, &b, msg)?)
    }

    pub fn acceptable_status(
        cfg: &StatusEvaluate,
        status: &Destinations<http::StatusCode>,
        _msg: &mut Vec<String>,
    ) -> WrappedResult<bool> {
        match cfg {
            StatusEvaluate::OkOrEqual => Ok(Self::assault_or_compare(status, http::StatusCode::is_success)),
        }
    }

    pub fn acceptable_header(
        cfg: &HeaderEvaluate,
        headers: &Destinations<http::HeaderMap>,
        _msg: &mut Vec<String>,
    ) -> WrappedResult<bool> {
        match cfg {
            HeaderEvaluate::Equal => Ok(Self::assault_or_compare(headers, |_| true)),
        }
    }

    pub fn acceptable_body(
        cfg: &BodyEvaluate,
        body: &Destinations<Bytes>,
        msg: &mut Vec<String>,
    ) -> WrappedResult<bool> {
        match cfg {
            BodyEvaluate::Equal => Ok(Self::assault_or_compare(body, |_| true)),
            BodyEvaluate::PlainText(_) => Ok(Self::assault_or_compare(body, |_| true)), // TODO
            #[cfg(feature = "json")]
            BodyEvaluate::Json(e) => Self::json_acceptable(e, body, msg),
        }
    }

    pub fn assault_or_compare<T: PartialEq, F: Fn(&T) -> bool>(d: &Destinations<T>, f: F) -> bool {
        if d.len() == 1 {
            Self::validate_all(d, f)
        } else {
            Self::compare_all(d)
        }
    }
    pub fn validate_all<T, F: Fn(&T) -> bool>(d: &Destinations<T>, f: F) -> bool {
        d.values().all(f)
    }
    pub fn compare_all<T: PartialEq>(status: &Destinations<T>) -> bool {
        let v: Vec<_> = status.values().collect();
        v.windows(2).all(|w| w[0] == w[1])
    }
}

#[cfg(feature = "json")]
impl DefaultEvaluator {
    pub fn json_acceptable(
        cfg: &JsonEvaluate,
        parts: &Destinations<Bytes>,
        msg: &mut Vec<String>,
    ) -> WrappedResult<bool> {
        let values: Vec<_> = match Self::patched(cfg, parts) {
            Ok(values) => values,
            Err(e) => {
                msg.push(format!("patch error: {}", e));
                return Ok(false);
            }
        }
        .into_values()
        .collect();

        let pass = values.windows(2).all(|w| Self::json_compare(cfg, (&w[0], &w[1]), msg).unwrap_or(w[0] == w[1]));
        Ok(pass)
    }

    pub fn patched(cfg: &JsonEvaluate, parts: &Destinations<Bytes>) -> WrappedResult<Destinations<Value>> {
        parts
            .iter()
            .map(|(name, body)| {
                let mut value = serde_json::from_slice(body)?;
                if let Err(e) = Self::patch(cfg, name, &mut value) {
                    if parts.len() == 1 {
                        Err(e)?;
                    }
                }
                Ok((name.clone(), value))
            })
            .collect::<Result<Destinations<_>, _>>()
    }
    pub fn patch(cfg: &JsonEvaluate, name: &str, value: &mut Value) -> Result<(), json_patch::PatchError> {
        let default_patch = json_patch::Patch::default();
        let patch = match &cfg.patch {
            Some(PatchTo::All(p)) => p,
            Some(PatchTo::Destinations(patch)) => patch.get(name).unwrap_or(&default_patch),
            None => &default_patch,
        };
        json_patch::patch(value, patch)
    }

    pub fn json_compare(cfg: &JsonEvaluate, (va, vb): (&Value, &Value), msg: &mut Vec<String>) -> WrappedResult<bool> {
        let diff = json_patch::diff(va, vb);
        let pointers = Self::pointers(&diff);
        for (op, path) in diff.iter().zip(pointers) {
            if cfg.ignore.contains(&path) {
                continue;
            } else {
                msg.push(format!("diff: {}", op));
                return Ok(false);
            }
        }
        Ok(true)
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
        let responses = Destinations::from_iter(vec![("test".to_string(), ok)]);
        let mut msg = Vec::new();
        let result = evaluator.evaluate(Default::default(), responses, &mut msg).await.unwrap();
        assert!(result);
        assert!(msg.is_empty());

        let unavailable = http::Response::builder()
            .status(http::StatusCode::SERVICE_UNAVAILABLE)
            .body(http_body_util::Empty::<Bytes>::new())
            .unwrap();
        let responses = Destinations::from_iter(vec![("test".to_string(), unavailable)]);
        let mut msg = Vec::new();
        let result = evaluator.evaluate(Default::default(), responses, &mut msg).await.unwrap();
        assert!(!result);
        assert!(msg.is_empty());
    }

    // TODO more tests
}
