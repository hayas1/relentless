use bytes::Bytes;
use regex::Regex;
use serde::{Deserialize, Serialize};

use crate::{
    error::EvaluateError,
    service::{
        destinations::{Destinations, AllOr},
        evaluator::Acceptable,
    },
};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(deny_unknown_fields, rename_all = "kebab-case")]
pub struct PlaintextEvaluate {
    pub regex: Option<AllOr<String>>,
}
impl Acceptable<Bytes> for PlaintextEvaluate {
    type Message = EvaluateError;
    fn accept(&self, bytes: &Destinations<Bytes>, msg: &mut Vec<Self::Message>) -> bool {
        let _ = msg; // TODO dest[d] can be failed
        match &self.regex {
            Some(AllOr::All(regex)) => Self::validate_all(bytes, |(_, b)| {
                Regex::new(regex).map(|re| re.is_match(String::from_utf8_lossy(b).as_ref())).unwrap_or(false)
            }),
            Some(AllOr::Destinations(dest)) => Self::validate_all(bytes, |(d, b)| {
                Regex::new(&dest[d]).map(|re| re.is_match(String::from_utf8_lossy(b).as_ref())).unwrap_or(false)
            }),
            None => true,
        }
    }
}
