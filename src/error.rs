use thiserror::Error;

pub type RelentlessResult<T, E = RelentlessError> = Result<T, E>;
pub type RelentlessResult_<T, E = RelentlessError_> = Result<T, E>;

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

// TODO derive macro
pub trait IntoRelentlessError: std::error::Error + Send + Sync + 'static {}

pub type RunCommandResult<T, E = RunCommandError> = RelentlessResult<T, E>;
#[derive(Error, Debug)]
pub enum RunCommandError {
    #[error(transparent)]
    Infallible(#[from] std::convert::Infallible),
    #[error("should be KEY=VALUE format, but `{0}` has no '='")]
    KeyValueFormat(String),
    #[error(transparent)]
    CannotParseAsString(#[from] Box<dyn std::error::Error + Send + Sync>),
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
