use std::fmt::{Display, Formatter};

pub type RelentlessResult<T> = Result<T, RelentlessError>;
#[derive(Debug)]
pub enum RelentlessError {
    CommandError(CommandError),
    Box(Box<dyn std::error::Error>),
}
impl std::error::Error for RelentlessError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            RelentlessError::CommandError(e) => Some(e),
            RelentlessError::Box(e) => e.source(),
        }
    }
}
impl Display for RelentlessError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            RelentlessError::CommandError(e) => e.fmt(f),
            RelentlessError::Box(e) => e.fmt(f),
        }
    }
}
impl RelentlessError {
    pub fn boxed<E: std::error::Error + 'static>(e: E) -> Self {
        RelentlessError::Box(Box::new(e))
    }
    pub fn error(&self) -> &(dyn std::error::Error + 'static) {
        match self {
            RelentlessError::CommandError(e) => e as _,
            RelentlessError::Box(e) => &**e,
        }
    }
}

#[derive(Debug)]
pub enum CommandError {
    InvalidKeyValueFormat { delim: char, got: String },
}
impl From<CommandError> for RelentlessError {
    fn from(value: CommandError) -> Self {
        RelentlessError::CommandError(value)
    }
}
impl std::error::Error for CommandError {}
impl Display for CommandError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            CommandError::InvalidKeyValueFormat { delim, got } => {
                write!(f, "argument is not in key{delim}value format: {got}")
            }
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
