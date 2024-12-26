use std::{
    error::Error as StdError,
    fmt::{Display, Result as FmtResult},
};

pub type RelentlessResult<T> = Result<T, RelentlessError>;
#[derive(Debug)]
pub struct RelentlessError {
    source: Box<dyn StdError + Send + Sync + 'static>,
}
impl StdError for RelentlessError {
    fn source(&self) -> Option<&(dyn StdError + 'static)> {
        Some(self.source.as_ref())
    }
}
impl Display for RelentlessError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> FmtResult {
        write!(f, "{}", self.source)
    }
}

#[derive(Debug)]
pub enum JsonEvaluateError {
    FailToPatchJson(json_patch::PatchError),
    FailToParseJson(serde_json::Error),
    Diff(String),
}
impl StdError for JsonEvaluateError {
    fn source(&self) -> Option<&(dyn StdError + 'static)> {
        match self {
            Self::FailToPatchJson(e) => Some(e),
            Self::FailToParseJson(e) => Some(e),
            Self::Diff(_) => None,
        }
    }
}
impl Display for JsonEvaluateError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> FmtResult {
        match self {
            Self::FailToPatchJson(e) => write!(f, "{}", e),
            Self::FailToParseJson(e) => write!(f, "{}", e),
            Self::Diff(e) => write!(f, "diff in `{}`", e),
        }
    }
}
