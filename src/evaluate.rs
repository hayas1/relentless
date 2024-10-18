use bytes::Bytes;
use http_body::Body;
use http_body_util::BodyExt;
#[cfg(feature = "json")]
use serde_json::Value;

#[cfg(feature = "json")]
use crate::config::{JsonEvaluate, PatchTo};
use crate::{
    config::{BodyEvaluate, Destinations},
    error::{Wrap, WrappedResult},
};

#[allow(async_fn_in_trait)] // TODO #[warn(async_fn_in_trait)] by default
pub trait Evaluator<Res> {
    type Error;
    type Message;
    async fn evaluate(
        &self,
        cfg: &BodyEvaluate,
        res: Destinations<Res>,
    ) -> Result<(bool, Option<Self::Message>), Self::Error>;
}
pub struct DefaultEvaluator;
impl<ResB: Body> Evaluator<http::Response<ResB>> for DefaultEvaluator
where
    ResB::Error: std::error::Error + Sync + Send + 'static,
{
    type Error = crate::Error;
    type Message = crate::Error;
    async fn evaluate(
        &self,
        cfg: &BodyEvaluate,
        res: Destinations<http::Response<ResB>>,
    ) -> Result<(bool, Option<Self::Message>), Self::Error> {
        let parts = Self::parts(res).await?;
        match cfg {
            BodyEvaluate::Nop | BodyEvaluate::PlainText(_) => match Self::acceptable(cfg, &parts).await {
                Ok(v) => Ok((v, None)),
                Err(err) => Ok((false, Some(err.into()))),
            },
            #[cfg(feature = "json")]
            BodyEvaluate::Json(_) => match Self::json_acceptable(cfg, &parts).await {
                Ok(v) => Ok((v, None)),
                Err(err) => {
                    if err.is::<json_patch::PatchError>() {
                        Ok((false, Some(err.into()))) // patch fail
                    } else {
                        Ok((Self::acceptable(cfg, &parts).await?, None))
                    }
                }
            },
        }
    }
}

impl DefaultEvaluator {
    pub async fn parts<ResB: Body>(
        res: Destinations<http::Response<ResB>>,
    ) -> Result<
        Destinations<(http::StatusCode, http::HeaderMap, Bytes)>,
        <Self as Evaluator<http::Response<ResB>>>::Error,
    >
    where
        ResB::Error: std::error::Error + Sync + Send + 'static,
    {
        let mut d = Destinations::new();
        for (name, r) in res {
            let (http::response::Parts { status, headers, .. }, body) = r.into_parts();
            let bytes = BodyExt::collect(body).await.map(|buf| buf.to_bytes()).map_err(Wrap::wrapping)?;
            d.insert(name, (status, headers, bytes));
        }
        Ok(d)
    }

    pub async fn acceptable(
        cfg: &BodyEvaluate,
        parts: &Destinations<(http::StatusCode, http::HeaderMap, Bytes)>,
    ) -> WrappedResult<bool> {
        if parts.len() == 1 {
            Self::status(parts).await
        } else {
            Self::compare(cfg, parts).await
        }
    }
    pub async fn status(parts: &Destinations<(http::StatusCode, http::HeaderMap, Bytes)>) -> WrappedResult<bool> {
        Ok(parts.iter().all(|(_name, (s, _h, _b))| s.is_success()))
    }
    pub async fn compare(
        _cfg: &BodyEvaluate,
        parts: &Destinations<(http::StatusCode, http::HeaderMap, Bytes)>,
    ) -> WrappedResult<bool> {
        let v: Vec<_> = parts.values().collect();
        let pass = v.windows(2).all(|w| w[0] == w[1]);
        Ok(pass)
    }
}

#[cfg(feature = "json")]
impl DefaultEvaluator {
    pub async fn json_acceptable(
        cfg: &BodyEvaluate,
        parts: &Destinations<(http::StatusCode, http::HeaderMap, Bytes)>,
    ) -> WrappedResult<bool> {
        let values = Self::patched(cfg, parts)?;

        let pass = parts.iter().zip(values.into_iter()).collect::<Vec<_>>().windows(2).all(|w| {
            let (((_na, (sa, ha, ba)), (__na, va)), ((_nb, (sb, hb, bb)), (__nb, vb))) = (&w[0], &w[1]);
            sa == sb && ha == hb && Self::json_compare(cfg, (va, vb)).unwrap_or(ba == bb)
        });
        Ok(pass)
    }

    pub fn patched(
        cfg: &BodyEvaluate,
        parts: &Destinations<(http::StatusCode, http::HeaderMap, Bytes)>,
    ) -> WrappedResult<Destinations<Value>> {
        parts
            .iter()
            .map(|(name, (_, _, body))| {
                let mut value = serde_json::from_slice(body)?;
                if let Err(e) = Self::patch(cfg, name, &mut value) {
                    if parts.len() == 1 {
                        Err(e)?;
                    } else {
                        eprintln!("patch was failed"); // TODO warning output
                    }
                }
                Ok((name.clone(), value))
            })
            .collect::<Result<Destinations<_>, _>>()
    }
    pub fn patch(cfg: &BodyEvaluate, name: &str, value: &mut Value) -> Result<(), json_patch::PatchError> {
        let patch = match cfg {
            BodyEvaluate::Json(JsonEvaluate { patch, .. }) => match patch {
                Some(PatchTo::All(p)) => p.clone(),
                Some(PatchTo::Destinations(patch)) => patch.get(name).cloned().unwrap_or_default(),
                None => json_patch::Patch::default(),
            },
            _ => json_patch::Patch::default(),
        };
        json_patch::patch(value, &patch)
    }

    pub fn json_compare(cfg: &BodyEvaluate, (va, vb): (&Value, &Value)) -> WrappedResult<bool> {
        let pointers = Self::pointers(&json_patch::diff(va, vb));
        let ignored = pointers.iter().all(|op| {
            match cfg {
                BodyEvaluate::Json(JsonEvaluate { ignore, .. }) => ignore.clone(),
                _ => Vec::new(),
            }
            .contains(op)
        });
        Ok(ignored)
    }

    pub fn pointers(p: &json_patch::Patch) -> Vec<String> {
        // TODO implemented ?
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
        let (result, msg) = evaluator.evaluate(&BodyEvaluate::Nop, responses).await.unwrap();
        assert!(matches!((result, msg), (true, None)));

        let unavailable = http::Response::builder()
            .status(http::StatusCode::SERVICE_UNAVAILABLE)
            .body(http_body_util::Empty::<Bytes>::new())
            .unwrap();
        let responses = Destinations::from_iter(vec![("test".to_string(), unavailable)]);
        let (result, msg) = evaluator.evaluate(&BodyEvaluate::Nop, responses).await.unwrap();
        assert!(matches!((result, msg), (false, None)));
    }

    // TODO more tests
}
