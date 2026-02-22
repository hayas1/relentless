use serde::{Deserialize, Serialize};

use crate::{
    error::EvaluateError,
    evaluator::evaluate::{Evaluator, Failure, Messages},
};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "kebab-case")]
pub struct ExpectEvaluator<T>(T);
impl<T> ExpectEvaluator<T> {
    pub fn new(expected: T) -> Self {
        Self(expected)
    }
    pub fn expected(&self) -> &T {
        &self.0
    }
}
impl<T: PartialEq<U>, U> Evaluator<U> for ExpectEvaluator<T> {
    // TODO T: Display for error message, but the status code (main use case), does not impl Display.
    type Message = EvaluateError;
    fn evaluate_shot(&self, msg: &mut Messages<Self::Message>, res: &U) -> Result<(), Failure> {
        // TODO error message
        self.evaluate_bool(msg, self.expected() == res, |_| EvaluateError::custom("unexpected"))
    }
    fn evaluate_compare(&self, msg: &mut Messages<Self::Message>, res1: &U, res2: &U) -> Result<(), Failure> {
        self.evaluate_bool(msg, self.expected() == res1, |_| EvaluateError::custom("unexpected"))?;
        self.evaluate_bool(msg, self.expected() == res2, |_| EvaluateError::custom("unexpected"))?;
        Ok(())
    }
}
