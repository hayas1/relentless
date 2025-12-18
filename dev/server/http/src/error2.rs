use std::fmt::Display;

use axum::{
    http::StatusCode,
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
    pub error: E,
}

#[derive(Error, Debug)]
pub struct AppError<E> {
    #[source]
    source: Box<dyn std::error::Error>,
    status: StatusCode,
    response: ErrorResponse<E>,
}
impl<E: Display + Serialize> IntoResponse for AppError<E> {
    fn into_response(self) -> Response {
        tracing::error!(
            source = %self.source,
            response = %self.response,
        );
        (self.status, Json(self.response)).into_response()
    }
}
pub trait IntoAppResult<T, R> {
    fn response(self, response: R) -> AppResult<T, R>;
}
impl<T, E, R> IntoAppResult<T, R> for Result<T, E>
where
    E: Into<Box<dyn std::error::Error + 'static>>,
    R: Serialize + AsStatusCode,
{
    fn response(self, response: R) -> AppResult<T, R> {
        self.map_err(|e| AppError {
            source: e.into(),
            status: response.status_code(),
            response: ErrorResponse { error: response },
        })
    }
}

pub trait AsStatusCode {
    fn status_code(&self) -> StatusCode {
        APP_DEFAULT_ERROR_CODE
    }
}
impl<T: Into<String>> AsStatusCode for T {}

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
        assert_eq!(external_err.source.to_string(), "error for server");
        assert_eq!(external_err.response.to_string(), "error for client");
        assert_eq!(external_err.status, APP_DEFAULT_ERROR_CODE);

        assert_eq!(
            br#"{"error":"ErrorResponse"}"#,
            &*body_bytes(external_err.into_response().into_body()).await.unwrap(),
        );
    }

    #[tokio::test]
    async fn test_error_message_response() {
        let internal = Err::<(), _>(Internal::InternalError);
        let external = internal.response("error for client");

        let external_err = external.unwrap_err();
        assert_eq!(external_err.source.to_string(), "error for server");
        assert_eq!(external_err.response.to_string(), "error for client");
        assert_eq!(external_err.status, APP_DEFAULT_ERROR_CODE);

        assert_eq!(
            br#"{"error":"error for client"}"#,
            &*body_bytes(external_err.into_response().into_body()).await.unwrap(),
        );
    }
}
