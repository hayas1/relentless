use std::{
    convert::Infallible,
    fmt::{Display, Formatter},
};

pub type RelentlessResult<T> = Result<T, RelentlessError>;
#[derive(Debug)]
pub enum RelentlessError {
    CommandError(CommandError),
    EvaluateError(EvaluateError),
    Box(Box<dyn std::error::Error + Send>),
    Custom(String),
}
impl std::error::Error for RelentlessError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::CommandError(e) => Some(e),
            Self::EvaluateError(e) => Some(e),
            Self::Box(e) => e.source(),
            Self::Custom(_) => None,
        }
    }
}
impl Display for RelentlessError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::CommandError(e) => e.fmt(f),
            Self::EvaluateError(e) => e.fmt(f),
            Self::Box(e) => e.fmt(f),
            Self::Custom(e) => e.fmt(f),
        }
    }
}
impl From<Infallible> for RelentlessError {
    fn from(value: Infallible) -> Self {
        match value {}
    }
}
impl RelentlessError {
    pub fn custom<T: Display>(e: T) -> Self {
        Self::Custom(e.to_string())
    }
    pub fn boxed<E: std::error::Error + Send + 'static>(e: E) -> Self {
        Self::Box(Box::new(e))
    }
    pub fn error(&self) -> &(dyn std::error::Error + 'static) {
        match self {
            Self::CommandError(e) => e as _,
            Self::EvaluateError(e) => e as _,
            Self::Box(e) => &**e,
            Self::Custom(_) => self,
        }
    }
}

#[derive(Debug)]
pub enum CommandError {
    InvalidKeyValueFormat { delim: char, got: String },
}
impl From<CommandError> for RelentlessError {
    fn from(value: CommandError) -> Self {
        Self::CommandError(value)
    }
}
impl std::error::Error for CommandError {}
impl Display for CommandError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidKeyValueFormat { delim, got } => {
                write!(f, "argument is not in key{delim}value format: {got}")
            }
        }
    }
}

#[derive(Debug)]
pub enum EvaluateError {
    EmptyTarget,
    ShouldCompare,
    ShouldShot,
    NotOk,
    Custom(String),
    Box(Box<dyn std::error::Error + Send + Sync + 'static>),
}
impl EvaluateError {
    pub fn custom<T: Display>(e: T) -> Self {
        Self::Custom(e.to_string())
    }
    pub fn boxed<E: Into<Box<dyn std::error::Error + Send + Sync + 'static>>>(e: E) -> Self {
        Self::Box(e.into())
    }
}
impl From<EvaluateError> for RelentlessError {
    fn from(value: EvaluateError) -> Self {
        Self::EvaluateError(value)
    }
}
impl std::error::Error for EvaluateError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Box(e) => e.source(),
            _ => None,
        }
    }
}
impl Display for EvaluateError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::EmptyTarget => write!(f, "empty target"),
            Self::ShouldCompare => write!(f, "should compare"),
            Self::ShouldShot => write!(f, "should shot"),
            Self::NotOk => write!(f, "not ok"),
            Self::Custom(e) => write!(f, "{e}"),
            Self::Box(e) => write!(f, "{e}"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_box_error_conversion() {
        fn f() -> crate::Result<()> {
            let result = Err(std::io::Error::other("test"));
            result.map_err(RelentlessError::boxed)?
        }
        let err = f().unwrap_err();
        assert!(matches!(err.error().downcast_ref().unwrap(), std::io::Error { .. }));
    }
}
