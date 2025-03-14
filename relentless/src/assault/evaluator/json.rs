use bytes::Bytes;
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::{
    assault::{
        destinations::{AllOr, Destinations},
        evaluate::Acceptable,
        messages::Messages,
    },
    error::JsonEvaluateError,
    interface::{config::Severity, helper::is_default::IsDefault},
};

#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "kebab-case")]
pub struct JsonEvaluator {
    #[serde(default, skip_serializing_if = "IsDefault::is_default")]
    pub ignore: Vec<String>,
    #[serde(default, skip_serializing_if = "IsDefault::is_default")]
    pub patch: Option<AllOr<json_patch::Patch>>,
    #[serde(default, skip_serializing_if = "IsDefault::is_default")]
    pub patch_fail: Option<Severity>,
}

impl Acceptable<&Bytes> for JsonEvaluator {
    type Message = JsonEvaluateError;
    fn accept(&self, bytes: &Destinations<&Bytes>, msg: &mut Messages<Self::Message>) -> bool {
        let Some(mut values) = msg.push_if_err(Self::to_values(bytes)) else {
            return false;
        };
        self.accept_json(&mut values, msg)
    }
}
impl Acceptable<&Value> for JsonEvaluator {
    type Message = JsonEvaluateError;
    fn accept(&self, values: &Destinations<&Value>, msg: &mut Messages<Self::Message>) -> bool {
        self.accept_json(&mut values.iter().map(|(k, &v)| (k, v.clone())).collect(), msg)
    }
}
impl JsonEvaluator {
    pub fn to_values(bytes: &Destinations<&Bytes>) -> Result<Destinations<Value>, JsonEvaluateError> {
        bytes
            .iter()
            .map(|(name, b)| {
                let value = serde_json::from_slice(b).map_err(JsonEvaluateError::FailToParseJson)?;
                Ok((name.clone(), value))
            })
            .collect()
    }
    pub fn accept_json(&self, values: &mut Destinations<Value>, msg: &mut Messages<JsonEvaluateError>) -> bool {
        let Some(patched) = msg.push_if_err(self.patched(values)) else {
            return false;
        };
        let values: Vec<_> = patched.values().collect();
        values.windows(2).all(|w| self.json_compare((w[0], w[1]), msg))
    }
    pub fn patched<'a>(
        &'a self,
        values: &'a mut Destinations<Value>,
    ) -> Result<Destinations<&'a mut Value>, JsonEvaluateError> {
        let is_assault = values.len() == 1;
        values
            .iter_mut()
            .map(|(name, v)| {
                if let Err(e) = self.patch(name, v) {
                    if self.patch_fail.is_none() && is_assault || self.patch_fail > Some(Severity::Warn) {
                        Err(JsonEvaluateError::FailToPatchJson(e))?;
                    }
                }
                Ok((name.clone(), v))
            })
            .collect()
    }
    pub fn patch(&self, name: &str, value: &mut Value) -> Result<(), json_patch::PatchError> {
        let default_patch = json_patch::Patch::default();
        let patch = match &self.patch {
            Some(AllOr::All(p)) => p,
            Some(AllOr::Destinations(patch)) => patch.get(name).unwrap_or(&default_patch),
            None => &default_patch,
        };
        json_patch::patch_unsafe(value, patch)
    }

    pub fn json_compare(&self, (va, vb): (&Value, &Value), msg: &mut Messages<JsonEvaluateError>) -> bool {
        let diff = json_patch::diff(va, vb);
        let pointers = Self::pointers(&diff);
        diff.iter()
            .zip(pointers)
            .filter(|(_op, path)| !self.ignore.contains(path))
            .fold(true, |_acc, (_op, path)| msg.push_unacceptable(JsonEvaluateError::Diff(path)))
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
