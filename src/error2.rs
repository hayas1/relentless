use std::{
    error::Error,
    fmt::{Display, Result as FmtResult},
    ops::{Deref, DerefMut},
};

use regex::Regex;

pub type RelentlessResult<T> = Result<T, RelentlessError>;
#[derive(Debug)]
pub struct RelentlessError {
    source: Box<dyn Error + Send + Sync + 'static>,
}
impl Deref for RelentlessError {
    type Target = Box<dyn Error + Send + Sync + 'static>;
    fn deref(&self) -> &Self::Target {
        &self.source
    }
}
impl DerefMut for RelentlessError {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.source
    }
}
impl Error for RelentlessError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        Some(self.source.as_ref())
    }
}
impl Display for RelentlessError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> FmtResult {
        write!(f, "{}", self.source)
    }
}
impl RelentlessError {
    pub fn boxed<E: Error + Send + Sync + 'static>(error: E) -> Self {
        RelentlessError { source: error.into() }
    }
    pub fn into_source(self) -> Box<dyn Error + Send + Sync> {
        // TODO is this method needed?
        self.source
    }
}

// TODO derive ?
pub trait IntoRelentlessError: Sized + Error + Send + Sync + 'static {
    fn into_relentless_error(self) -> RelentlessError {
        RelentlessError { source: Box::new(self) }
    }
}
impl<E: Error + Send + Sync + 'static> IntoRelentlessError for Box<E> {
    fn into_relentless_error(self) -> RelentlessError {
        RelentlessError { source: self }
    }
}
impl<E: IntoRelentlessError> From<E> for RelentlessError {
    fn from(e: E) -> Self {
        e.into_relentless_error()
    }
}

#[derive(Debug)]
pub enum AssaultError {
    CannotSpecifyService,
}
impl IntoRelentlessError for AssaultError {}
impl Error for AssaultError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::CannotSpecifyService => None,
        }
    }
}
impl Display for AssaultError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> FmtResult {
        match self {
            Self::CannotSpecifyService => write!(f, "cannot specify service"),
        }
    }
}

#[derive(Debug)]
pub enum PlaintextEvaluateError {
    FailToCompileRegex(regex::Error),
    FailToMatch(Regex, String),
}
impl IntoRelentlessError for PlaintextEvaluateError {}
impl Error for PlaintextEvaluateError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
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
impl Error for JsonEvaluateError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
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
