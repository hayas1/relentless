use std::{
    error::Error as StdError,
    fmt::{Display, Result as FmtResult},
};

use crate::error::{EvaluateError, Wrap};

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
    fn context<C: Into<ErrorContext>>(self, context: C) -> RelentlessResult<T>;
}
impl<T, E: StdError + Send + Sync + 'static> Context<T> for Result<T, E> {
    fn context<C: Into<ErrorContext>>(self, context: C) -> RelentlessResult<T> {
        self.map_err(|e| RelentlessError { source: Some(Box::new(e)), context: context.into() })
    }
}
impl<T> Context<T> for Result<T, Wrap> {
    fn context<C: Into<ErrorContext>>(self, context: C) -> RelentlessResult<T> {
        self.map_err(|e| RelentlessError { source: Some(e.source()), context: context.into() })
    }
}
impl<T> Context<T> for Option<T> {
    fn context<C: Into<ErrorContext>>(self, context: C) -> RelentlessResult<T> {
        self.ok_or(RelentlessError { source: None, context: context.into() })
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
impl From<EvaluateError> for ErrorContext {
    fn from(_: EvaluateError) -> Self {
        Self::Evaluate
    }
}
