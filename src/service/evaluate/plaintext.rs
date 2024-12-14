use bytes::Bytes;
use regex::Regex;
use serde::{Deserialize, Serialize};

use crate::{error::EvaluateError, interface::config::EvaluateTo, service::destinations::Destinations};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(deny_unknown_fields, rename_all = "kebab-case")]
pub struct PlaintextEvaluate {
    pub regex: Option<EvaluateTo<String>>,
}
impl PlaintextEvaluate {
    pub fn plaintext_acceptable(&self, bytes: &Destinations<Bytes>, msg: &mut Vec<EvaluateError>) -> bool {
        let _ = msg; // TODO body[d] can be failed
        match &self.regex {
            Some(EvaluateTo::All(regex)) => bytes.iter().all(|(_, b)| {
                Regex::new(regex).map(|re| re.is_match(String::from_utf8_lossy(b).as_ref())).unwrap_or(false)
            }),
            Some(EvaluateTo::Destinations(dest)) => dest.iter().all(|(d, regex)| {
                Regex::new(regex)
                    .map(|re| re.is_match(String::from_utf8_lossy(bytes[d].as_ref()).as_ref()))
                    .unwrap_or(false)
            }),
            None => true,
        }
    }
}
