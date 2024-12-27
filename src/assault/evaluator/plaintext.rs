use bytes::Bytes;
use regex::Regex;
use serde::{Deserialize, Serialize};

use crate::{
    assault::{
        destinations::{AllOr, Destinations},
        evaluate::Acceptable,
        messages::Messages,
    },
    error2::PlaintextEvaluateError,
};

#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "kebab-case")]
pub struct PlaintextEvaluator {
    pub regex: Option<AllOr<String>>,
}
impl Acceptable<&Bytes> for PlaintextEvaluator {
    type Message = PlaintextEvaluateError;
    fn accept(&self, bytes: &Destinations<&Bytes>, msg: &mut Messages<Self::Message>) -> bool {
        let _ = msg; // TODO dest[d] can be failed
        match &self.regex {
            Some(AllOr::All(regex)) => Self::validate_all(bytes, |(_, b)| {
                msg.push_if_err(Self::is_match(regex, &String::from_utf8_lossy(b))).is_some()
            }),
            Some(AllOr::Destinations(dest)) => Self::validate_all(bytes, |(d, b)| {
                msg.push_if_err(Self::is_match(&dest[d], &String::from_utf8_lossy(b))).is_some()
            }),
            None => true,
        }
    }
}

impl PlaintextEvaluator {
    pub fn is_match(regex: &str, haystack: &str) -> Result<(), PlaintextEvaluateError> {
        let re = Regex::new(regex).map_err(PlaintextEvaluateError::FailToCompileRegex)?;
        re.is_match(haystack).then_some(()).ok_or(PlaintextEvaluateError::FailToMatch(re, haystack.to_string()))
    }
}
