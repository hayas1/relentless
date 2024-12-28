use std::{
    error::Error,
    fmt::Display,
    ops::{Deref, DerefMut},
    task::Poll,
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
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
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

// From<Error> implementation will be conflict (similar issue https://github.com/dtolnay/anyhow/issues/25#issuecomment-544140480)
pub trait IntoResult<T> {
    type Result;
    fn box_err(self) -> Self::Result;
}
impl<T, E: Error + Send + Sync + 'static> IntoResult<T> for Result<T, E> {
    type Result = RelentlessResult<T>;
    fn box_err(self) -> Self::Result {
        self.map_err(RelentlessError::boxed)
    }
}
impl<T, E: Error + Send + Sync + 'static> IntoResult<T> for Poll<Result<T, E>> {
    type Result = Poll<RelentlessResult<T>>;
    fn box_err(self) -> Self::Result {
        self.map_err(RelentlessError::boxed)
    }
}

#[derive(Debug)]
pub enum InterfaceError {
    UndefinedSerializeFormatPath(String),
    UndefinedSerializeFormatContent(String),
    KeyValueFormat(String),
    UnknownFormatExtension(String),
    CannotReadConfig(String, RelentlessError),
    CannotSpecifyFormat,
    NanPercentile,
}
impl IntoRelentlessError for InterfaceError {}
impl Error for InterfaceError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::UndefinedSerializeFormatPath(_) => None,
            Self::UndefinedSerializeFormatContent(_) => None,
            Self::KeyValueFormat(_) => None,
            Self::UnknownFormatExtension(_) => None,
            Self::CannotReadConfig(_, e) => Some(e),
            Self::CannotSpecifyFormat => None,
            Self::NanPercentile => None,
        }
    }
}
impl Display for InterfaceError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::UndefinedSerializeFormatPath(s) => {
                write!(f, "no serde format is enabled for path `{}`", s)
            }
            Self::UndefinedSerializeFormatContent(s) => {
                write!(f, "no serde format is enabled for content `{}`", s)
            }
            Self::KeyValueFormat(s) => write!(f, "should be KEY=VALUE format, but `{}` has no `=`", s),
            Self::UnknownFormatExtension(s) => write!(f, "`{}` is unknown extension format", s),
            Self::CannotReadConfig(s, e) => write!(f, "[{}] {}", s, e),
            Self::CannotSpecifyFormat => write!(f, "cannot specify format"),
            Self::NanPercentile => write!(f, "nan is not number"),
        }
    }
}

#[derive(Debug)]
pub enum TemplateError {
    NomParseError(String),
    RemainingTemplate(String),
    VariableNotDefined(String),
}
impl IntoRelentlessError for TemplateError {}
impl Error for TemplateError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::NomParseError(_) => None,
            Self::RemainingTemplate(_) => None,
            Self::VariableNotDefined(_) => None,
        }
    }
}
impl Display for TemplateError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NomParseError(s) => write!(f, "{}", s),
            Self::RemainingTemplate(s) => write!(f, "remaining template: {}", s),
            Self::VariableNotDefined(s) => write!(f, "variable `{}` is not defined", s),
        }
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
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
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
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
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
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::FailToPatchJson(e) => write!(f, "{}", e),
            Self::FailToParseJson(e) => write!(f, "{}", e),
            Self::Diff(e) => write!(f, "diff in `{}`", e),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_conversion() {
        fn f() -> RelentlessResult<()> {
            Err(InterfaceError::UndefinedSerializeFormatPath("test".to_string()))?
        }
        let err = f().unwrap_err();
        assert!(matches!(err.downcast_ref().unwrap(), InterfaceError::UndefinedSerializeFormatPath(s) if s == "test"));
    }

    #[test]
    fn test_box_error_conversion() {
        fn f() -> RelentlessResult<()> {
            Err(std::io::Error::new(std::io::ErrorKind::Other, "test")).box_err()?
        }
        let err = f().unwrap_err();
        assert!(matches!(err.downcast_ref().unwrap(), std::io::Error { .. }));
    }
}
