use bytes::Bytes;
use regex::Regex;
use serde::{Deserialize, Serialize};

use crate::{
    assault::{
        destinations::{AllOr, Destinations},
        evaluate::Acceptable,
        messages::Messages,
    },
    error::EvaluateError,
};

#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "kebab-case")]
pub struct PlaintextEvaluator {
    pub regex: Option<AllOr<String>>,
}
impl Acceptable<&Bytes> for PlaintextEvaluator {
    type Message = EvaluateError;
    fn accept(&self, bytes: &Destinations<&Bytes>, msg: &mut Messages<Self::Message>) -> bool {
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
