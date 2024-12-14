use bytes::Bytes;
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::{
    error::{EvaluateError, WrappedResult},
    interface::{config::Severity, helper::is_default::IsDefault},
    service::{
        destinations::{Destinations, EvaluateTo},
        evaluator::Acceptable,
    },
};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(deny_unknown_fields, rename_all = "kebab-case")]
pub struct JsonEvaluate {
    #[serde(default, skip_serializing_if = "IsDefault::is_default")]
    pub ignore: Vec<String>,
    #[serde(default, skip_serializing_if = "IsDefault::is_default")]
    pub patch: Option<EvaluateTo<json_patch::Patch>>,
    #[serde(default, skip_serializing_if = "IsDefault::is_default")]
    pub patch_fail: Option<Severity>,
}

#[cfg(feature = "json")]
impl Acceptable<Bytes> for JsonEvaluate {
    type Message = EvaluateError;
    fn accept(&self, bytes: &Destinations<Bytes>, msg: &mut Vec<EvaluateError>) -> bool {
        self.accept_json(bytes, msg)
    }
}
impl JsonEvaluate {
    pub fn accept_json(&self, bytes: &Destinations<Bytes>, msg: &mut Vec<EvaluateError>) -> bool {
        let values: Vec<_> = match self.patched(bytes) {
            Ok(values) => values,
            Err(e) => {
                msg.push(EvaluateError::FailToPatchJson(e));
                return false;
            }
        }
        .into_values()
        .collect();

        values.windows(2).all(|w| self.json_compare((&w[0], &w[1]), msg))
    }

    pub fn patched(&self, bytes: &Destinations<Bytes>) -> WrappedResult<Destinations<Value>> {
        bytes
            .iter()
            .map(|(name, b)| {
                let mut value = serde_json::from_slice(b)?;
                if let Err(e) = self.patch(name, &mut value) {
                    if self.patch_fail.is_none() && bytes.len() == 1 || self.patch_fail > Some(Severity::Warn) {
                        Err(e)?;
                    }
                }
                Ok((name.clone(), value))
            })
            .collect()
    }
    pub fn patch(&self, name: &str, value: &mut Value) -> Result<(), json_patch::PatchError> {
        let default_patch = json_patch::Patch::default();
        let patch = match &self.patch {
            Some(EvaluateTo::All(p)) => p,
            Some(EvaluateTo::Destinations(patch)) => patch.get(name).unwrap_or(&default_patch),
            None => &default_patch,
        };
        json_patch::patch_unsafe(value, patch)
    }

    pub fn json_compare(&self, (va, vb): (&Value, &Value), msg: &mut Vec<EvaluateError>) -> bool {
        let diff = json_patch::diff(va, vb);
        let pointers = Self::pointers(&diff);
        diff.iter().zip(pointers).filter(|(_op, path)| !self.ignore.contains(path)).fold(true, |_acc, (_op, path)| {
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
