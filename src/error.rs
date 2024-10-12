use std::{
    fmt::{Debug, Display},
    ops::{Deref, DerefMut},
};

use thiserror::Error;

use crate::config::Config;

pub type RelentlessResult<T, E = RelentlessError> = Result<T, E>;
pub type RelentlessResult_<T, E = Wrap> = Result<T, E>;

#[derive(Error, Debug)]
#[error(transparent)]
pub struct RelentlessError_ {
    #[from]
    source: Box<dyn std::error::Error + Send + Sync>,
}
impl<T: IntoRelentlessError> From<T> for RelentlessError_ {
    fn from(e: T) -> Self {
        RelentlessError_ { source: Box::new(e) }
    }
}
impl From<Wrap> for RelentlessError_ {
    fn from(wrap: Wrap) -> Self {
        RelentlessError_ { source: wrap.0 }
    }
}
impl RelentlessError_ {
    pub fn is<E: IntoRelentlessError>(&self) -> bool {
        self.source.is::<E>()
    }
    pub fn downcast<E: IntoRelentlessError>(self) -> Result<Box<E>, Box<dyn std::error::Error + Send + Sync>> {
        self.source.downcast()
    }
    pub fn downcast_ref<E: IntoRelentlessError>(&self) -> Option<&E> {
        self.source.downcast_ref()
    }
    pub fn downcast_mut<E: IntoRelentlessError>(&mut self) -> Option<&mut E> {
        self.source.downcast_mut()
    }
}

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
    pub fn source(self) -> Box<dyn std::error::Error + Send + Sync> {
        self.0
    }
    pub fn context<T>(self, context: T) -> Context<T> {
        Context { context, source: self.0 }
    }
    pub fn is<E: IntoRelentlessError>(&self) -> bool {
        self.0.is::<E>()
    }
    pub fn downcast<E: IntoRelentlessError>(self) -> Result<Box<E>, Box<dyn std::error::Error + Send + Sync>> {
        self.0.downcast()
    }
    pub fn downcast_ref<E: IntoRelentlessError>(&self) -> Option<&E> {
        self.0.downcast_ref()
    }
    pub fn downcast_mut<E: IntoRelentlessError>(&mut self) -> Option<&mut E> {
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

pub type RunCommandResult<T, E = RunCommandError> = RelentlessResult<T, E>;
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

pub type AssaultResult<T, E = AssaultError> = RelentlessResult<T, E>;
#[derive(Error, Debug)]
pub enum AssaultError {}
impl IntoRelentlessError for AssaultError {}

pub type EvaluateResult<T, E = EvaluateError> = RelentlessResult<T, E>;
#[derive(Error, Debug)]
pub enum EvaluateError {}
impl IntoRelentlessError for EvaluateError {}

pub type ReportResult<T, E = ReportError> = RelentlessResult<T, E>;
#[derive(Error, Debug)]
pub enum ReportError {}
impl IntoRelentlessError for ReportError {}

#[derive(Error, Debug)]
#[error(transparent)]
pub enum RelentlessError {
    RelentlessError_(#[from] RelentlessError_),

    FormatError(#[from] FormatError),
    HttpError(#[from] HttpError),
    CaseError(#[from] CaseError),

    #[cfg(feature = "default-http-client")]
    ReqwestError(#[from] reqwest::Error),
    HttpInvalidUri(#[from] http::uri::InvalidUri),
    TokioTaskJoinError(#[from] tokio::task::JoinError),
    StdFmtError(#[from] std::fmt::Error),
    Infallible(#[from] std::convert::Infallible),

    #[cfg(feature = "json")]
    JsonPatchError(#[from] json_patch::PatchError),

    #[cfg(feature = "json")]
    JsonError(#[from] JsonError),

    BoxError(#[from] Box<dyn std::error::Error + Send + Sync>),
}
impl IntoRelentlessError for RelentlessError {}
// impl<T: IntoRelentlessError> From<T> for RelentlessError {
//     fn from(e: T) -> Self {
//         RelentlessError::RelentlessError_(e.into())
//     }
// }

#[derive(Error, Debug)]
pub enum FormatError {
    #[error("unknown format extension: {0}")]
    UnknownFormatExtension(String),
    #[error("cannot specify format")]
    CannotSpecifyFormat,

    #[error(transparent)]
    StdIoError(#[from] std::io::Error),
    #[cfg(feature = "json")]
    #[error(transparent)]
    JsonError(#[from] serde_json::Error),
    #[cfg(feature = "yaml")]
    #[error(transparent)]
    YamlError(#[from] serde_yaml::Error),
    #[cfg(feature = "toml")]
    #[error(transparent)]
    TomlError(#[from] toml::de::Error),
}

#[derive(Error, Debug)]
pub enum HttpError {
    #[error("cannot convert body")]
    CannotConvertBody,

    #[error(transparent)]
    InvalidMethod(#[from] http::method::InvalidMethod),
}

#[derive(Error, Debug)]
pub enum CaseError {
    #[error("fail to clone request")]
    FailCloneRequest,
}

#[cfg(feature = "json")]
#[derive(Error, Debug)]
pub enum JsonError {
    #[error("fail to patch json")]
    FailToPatch,
}
