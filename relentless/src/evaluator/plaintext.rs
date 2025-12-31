use regex::Regex;
use serde::{Deserialize, Serialize};

use crate::{
    error::EvaluateError,
    evaluator::evaluate::{Evaluator, Failure, Messages},
};

#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "kebab-case")]
pub struct RegexEvaluator(String);
impl RegexEvaluator {
    pub fn raw_regex(&self) -> &str {
        &self.0
    }
    pub fn regex(&self) -> Result<Regex, regex::Error> {
        self.raw_regex().parse()
    }
    pub fn is_match(&self, haystack: &str) -> Result<bool, regex::Error> {
        Ok(self.regex()?.is_match(haystack))
    }
}
impl Evaluator<str> for RegexEvaluator {
    type Message = EvaluateError;
    fn evaluate_shot(&self, msg: &mut Messages<Self::Message>, res: &str) -> Result<(), Failure> {
        let regex = self.regex().map_err(|e| msg.error(EvaluateError::boxed(e)))?;
        self.evaluate_bool(msg, regex.is_match(res), |_| EvaluateError::custom("not match"))
    }
    fn evaluate_compare(&self, msg: &mut Messages<Self::Message>, res1: &str, res2: &str) -> Result<(), Failure> {
        let regex = self.regex().map_err(|e| msg.error(EvaluateError::boxed(e)))?;
        self.evaluate_bool(msg, regex.is_match(res1), |_| EvaluateError::custom("not match"))?;
        self.evaluate_bool(msg, regex.is_match(res2), |_| EvaluateError::custom("not match"))?;
        Ok(())
    }
}
