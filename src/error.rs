use std::{
    fmt::{Debug, Display},
    ops::{Deref, DerefMut},
};

use thiserror::Error;

use crate::config::Config;

pub type RelentlessResult<T, E = RelentlessError> = Result<T, E>;

#[derive(Error, Debug)]
#[error(transparent)]
pub struct RelentlessError {
    #[from]
    source: Box<dyn std::error::Error + Send + Sync>,
}
impl<T: IntoRelentlessError> From<T> for RelentlessError {
    fn from(e: T) -> Self {
        RelentlessError { source: Box::new(e) }
    }
}
impl From<Wrap> for RelentlessError {
    fn from(wrap: Wrap) -> Self {
        RelentlessError { source: wrap.0 }
    }
}
impl RelentlessError {
    pub fn is<E: std::error::Error + Send + Sync + 'static>(&self) -> bool {
        self.source.is::<E>()
    }
    pub fn downcast<E: std::error::Error + Send + Sync + 'static>(
        self,
    ) -> Result<Box<E>, Box<dyn std::error::Error + Send + Sync>> {
        self.source.downcast()
    }
    pub fn downcast_ref<E: std::error::Error + Send + Sync + 'static>(&self) -> Option<&E> {
        self.source.downcast_ref()
    }
    pub fn downcast_mut<E: std::error::Error + Send + Sync + 'static>(&mut self) -> Option<&mut E> {
        self.source.downcast_mut()
    }
}

pub type WrappedResult<T, E = Wrap> = Result<T, E>;

#[derive(Debug)]
pub struct Wrap(pub Box<dyn std::error::Error + Send + Sync>);
impl<E: std::error::Error + Send + Sync + 'static> From<E> for Wrap {
    fn from(e: E) -> Self {
        Self(Box::new(e))
    }
}
impl Display for Wrap {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        Display::fmt(&*self.0, f)
    }
}
impl Deref for Wrap {
    type Target = Box<dyn std::error::Error + Send + Sync>;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}
impl DerefMut for Wrap {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}
impl Wrap {
    pub fn new(e: Box<dyn std::error::Error + Send + Sync>) -> Self {
        Self(e)
    }
    pub fn wrapping<E: std::error::Error + Send + Sync + 'static>(e: E) -> Self {
        Self::from(e)
    }
    pub fn error<E: std::error::Error + Send + Sync + 'static>(e: E) -> crate::Error {
        Self::from(e).into()
    }

    pub fn source(self) -> Box<dyn std::error::Error + Send + Sync> {
        self.0
    }
    pub fn context<T>(self, context: T) -> Context<T> {
        Context { context, source: self.0 }
    }
    pub fn is<E: std::error::Error + Send + Sync + 'static>(&self) -> bool {
        self.0.is::<E>()
    }
    pub fn downcast<E: std::error::Error + Send + Sync + 'static>(
        self,
    ) -> Result<Box<E>, Box<dyn std::error::Error + Send + Sync>> {
        self.0.downcast()
    }
    pub fn downcast_ref<E: std::error::Error + Send + Sync + 'static>(&self) -> Option<&E> {
        self.0.downcast_ref()
    }
    pub fn downcast_mut<E: std::error::Error + Send + Sync + 'static>(&mut self) -> Option<&mut E> {
        self.0.downcast_mut()
    }
}

pub trait IntoContext: std::error::Error + Send + Sync + 'static + Sized {
    fn context<T>(self, context: T) -> Context<T> {
        Context { context, source: Box::new(self) }
    }
}
impl<E: std::error::Error + Send + Sync + 'static> IntoContext for E {}
#[derive(Debug)]
pub struct Context<T> {
    context: T,
    source: Box<dyn std::error::Error + Send + Sync>,
}
impl<T: Display + Debug + Send + Sync + 'static> IntoRelentlessError for Context<T> {}
impl<T: Display + Debug> std::error::Error for Context<T> {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        Some(self.source.as_ref())
    }
}
impl<T: Display> Display for Context<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "{}: {}", self.context, self.source)
    }
}

// TODO derive macro
pub trait IntoRelentlessError: std::error::Error + Send + Sync + 'static {}

#[derive(Error, Debug)]
pub enum RunCommandError {
    #[error("should be KEY=VALUE format, but `{0}` has no '='")]
    KeyValueFormat(String),
    #[error("unknown format extension: {0}")]
    UnknownFormatExtension(String),
    #[error("cannot read some configs: {1:?}")]
    CannotReadSomeConfigs(Vec<Config>, Vec<Wrap>),
    #[error("cannot specify format")]
    CannotSpecifyFormat,
}
impl IntoRelentlessError for RunCommandError {}

#[derive(Error, Debug)]
pub enum AssaultError {}
impl IntoRelentlessError for AssaultError {}

#[derive(Error, Debug)]
pub enum EvaluateError {}
impl IntoRelentlessError for EvaluateError {}

#[derive(Error, Debug)]
pub enum ReportError {}
impl IntoRelentlessError for ReportError {}
