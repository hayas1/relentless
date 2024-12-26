use std::{
    error::Error as StdError,
    fmt::{Display, Result as FmtResult},
};

pub type RelentlessResult<T> = Result<T, RelentlessError>;
#[derive(Debug)]
pub struct RelentlessError {
    source: Option<Box<dyn StdError + Send + Sync + 'static>>,
    context: ErrorContext,
}
impl StdError for RelentlessError {
    fn source(&self) -> Option<&(dyn StdError + 'static)> {
        self.source.as_ref().map(|e| &**e as _)
    }
}
impl Display for RelentlessError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> FmtResult {
        if let Some(source) = &self.source {
            write!(f, "{}: {}", self.context, source)
        } else {
            write!(f, "{}", self.context)
        }
    }
}

pub trait Context<T> {
    fn context(self, context: ErrorContext) -> Result<T, RelentlessError>;
}
impl<T, E: StdError + Send + Sync + 'static> Context<T> for Result<T, E> {
    fn context(self, context: ErrorContext) -> Result<T, RelentlessError> {
        self.map_err(|e| RelentlessError { source: Some(Box::new(e)), context })
    }
}
impl<T> Context<T> for Option<T> {
    fn context(self, context: ErrorContext) -> Result<T, RelentlessError> {
        self.ok_or(RelentlessError { source: None, context })
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum ErrorContext {
    RunCommand,
    Template,
    Assault,
    Evaluate,
}
impl Display for ErrorContext {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ErrorContext::RunCommand => write!(f, "run command error"),
            ErrorContext::Template => write!(f, "template error"),
            ErrorContext::Assault => write!(f, "assault error"),
            ErrorContext::Evaluate => write!(f, "evaluate error"),
        }
    }
}
