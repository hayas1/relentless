use std::{
    error::Error as StdError,
    fmt::{Display, Result as FmtResult},
};

use regex::Regex;

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

// TODO derive ?
pub trait IntoRelentlessError: Sized + StdError + Send + Sync + 'static {
    fn into_relentless_error(self) -> RelentlessError {
        RelentlessError { source: Box::new(self) }
    }
}
impl<E: IntoRelentlessError> From<E> for RelentlessError {
    fn from(e: E) -> Self {
        e.into_relentless_error()
    }
}

#[derive(Debug)]
pub enum PlaintextEvaluateError {
    FailToCompileRegex(regex::Error),
    FailToMatch(Regex, String),
}
impl IntoRelentlessError for PlaintextEvaluateError {}
impl StdError for PlaintextEvaluateError {
    fn source(&self) -> Option<&(dyn StdError + 'static)> {
        match self {
            Self::FailToCompileRegex(e) => Some(e),
            Self::FailToMatch(_, _) => None,
        }
    }
}
impl Display for PlaintextEvaluateError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> FmtResult {
        match self {
            Self::FailToCompileRegex(e) => write!(f, "{}", e),
            Self::FailToMatch(re, haystack) => write!(f, "regex `{}` does not match `{}`", re, haystack),
        }
    }
}

#[derive(Debug)]
#[cfg(feature = "json")]
pub enum JsonEvaluateError {
    FailToPatchJson(json_patch::PatchError),
    FailToParseJson(serde_json::Error),
    Diff(String),
}
#[cfg(feature = "json")]
impl IntoRelentlessError for JsonEvaluateError {}
#[cfg(feature = "json")]
impl StdError for JsonEvaluateError {
    fn source(&self) -> Option<&(dyn StdError + 'static)> {
        match self {
            Self::FailToPatchJson(e) => Some(e),
            Self::FailToParseJson(e) => Some(e),
            Self::Diff(_) => None,
        }
    }
}
#[cfg(feature = "json")]
impl Display for JsonEvaluateError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> FmtResult {
        match self {
            Self::FailToPatchJson(e) => write!(f, "{}", e),
            Self::FailToParseJson(e) => write!(f, "{}", e),
            Self::Diff(e) => write!(f, "diff in `{}`", e),
        }
    }
}
