use std::fmt::{Debug, Display};

use axum::{
    http::{StatusCode, Uri},
    response::{IntoResponse, Response},
    Json,
};
use serde::{Deserialize, Serialize};
use thiserror::Error;

pub const APP_DEFAULT_ERROR_CODE: StatusCode = StatusCode::BAD_REQUEST;

pub type AppResult<T, E = String> = Result<T, AppError<E>>;
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Default, Hash, Serialize, Deserialize, Error)]
#[error("{error}")]
pub struct ErrorResponse<E = String> {
    pub display: String,
    pub error: Option<E>,
}
impl<E: Display> From<E> for ErrorResponse<E> {
    fn from(error: E) -> Self {
        Self { display: error.to_string(), error: Some(error) }
    }
}
impl ErrorResponse {
    pub fn new<S: Into<String>>(s: S) -> Self {
        Self { display: s.into(), error: None }
    }
}

#[derive(Error, Debug)]
pub struct AppError<E> {
    #[source]
    source: Option<Box<dyn std::error::Error>>,
    status: StatusCode,
    response: ErrorResponse<E>,
}
impl<E> AppError<E> {
    pub fn new(response: E) -> Self
    where
        E: AsStatusCode + Display,
    {
        let status = response.status_code();
        Self { source: None, status, response: response.into() }
    }
    // pub fn response_into<F: From<E>>(self) -> AppError<F> {
    //     let response = ErrorResponse { error: self.response.error.into() };
    //     AppError { source: self.source, status: self.status, response }
    // }
}
impl<E: Debug + Serialize> IntoResponse for AppError<E> {
    fn into_response(self) -> Response {
        tracing::error!(
            source = ?self.source,
            message = %self.response.display,
            response = ?self.response,
        );
        (self.status, Json(self.response)).into_response()
    }
}
pub trait IntoAppResult<T, E, R> {
    fn response(self, error: R) -> AppResult<T, R>;
    fn response_map(self, f: impl FnOnce(&E) -> R) -> AppResult<T, R>;
}
impl<T, E, R> IntoAppResult<T, E, R> for Result<T, E>
where
    E: Into<Box<dyn std::error::Error + 'static>>,
    R: Display + Serialize + AsStatusCode,
{
    fn response(self, error: R) -> AppResult<T, R> {
        self.map_err(|e| AppError { source: Some(e.into()), status: error.status_code(), response: error.into() })
    }
    fn response_map(self, f: impl FnOnce(&E) -> R) -> AppResult<T, R> {
        self.map_err(|e| {
            let error = f(&e);
            AppError { source: Some(e.into()), status: error.status_code(), response: error.into() }
        })
    }
}

pub trait AsStatusCode {
    fn status_code(&self) -> StatusCode {
        APP_DEFAULT_ERROR_CODE
    }
}
impl<T: Into<String>> AsStatusCode for T {}

#[derive(Debug, Error, Serialize, Deserialize)]
#[error("not found: {uri}")]
pub struct NotFound {
    uri: String,
}
impl NotFound {
    pub fn new(uri: Uri) -> Self {
        Self { uri: uri.to_string() }
    }
}
impl AsStatusCode for NotFound {
    fn status_code(&self) -> StatusCode {
        StatusCode::NOT_FOUND
    }
}

#[derive(Debug, Error, Serialize, Deserialize)]
#[error("please try again later")]
pub struct Retriable;
impl AsStatusCode for Retriable {
    fn status_code(&self) -> StatusCode {
        StatusCode::SERVICE_UNAVAILABLE
    }
}

#[cfg(test)]
mod tests {
    use crate::app::tests::body_bytes;

    use super::*;

    #[derive(Error, Debug)]
    enum Internal {
        #[error("error for server")]
        InternalError,
    }
    #[derive(Error, Debug, Serialize, Deserialize)]
    enum External {
        #[error("error for client")]
        ErrorResponse,
    }
    impl AsStatusCode for External {}

    #[tokio::test]
    async fn test_error_response() {
        let internal = Err::<(), _>(Internal::InternalError);
        let external = internal.response(External::ErrorResponse);

        let external_err = external.unwrap_err();
        assert_eq!(external_err.source.as_ref().unwrap().to_string(), "error for server");
        assert_eq!(external_err.response.display, "error for client");
        assert_eq!(external_err.status, APP_DEFAULT_ERROR_CODE);

        assert_eq!(
            br#"{"display":"error for client","error":"ErrorResponse"}"#,
            &*body_bytes(external_err.into_response().into_body()).await.unwrap(),
        );
    }

    #[tokio::test]
    async fn test_error_message_response() {
        let internal = Err::<(), _>(Internal::InternalError);
        let external = internal.response("error for client");

        let external_err = external.unwrap_err();
        assert_eq!(external_err.source.as_ref().unwrap().to_string(), "error for server");
        assert_eq!(external_err.response.display, "error for client");
        assert_eq!(external_err.status, APP_DEFAULT_ERROR_CODE);

        assert_eq!(
            br#"{"display":"error for client","error":"error for client"}"#,
            &*body_bytes(external_err.into_response().into_body()).await.unwrap(),
        );
    }
}
