use std::fmt::Display;

use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use serde::{Deserialize, Serialize};
use thiserror::Error;

pub const APP_DEFAULT_ERROR_CODE: StatusCode = StatusCode::BAD_REQUEST;

pub type AppResult<T, E = ErrorResponse> = Result<T, AppError<E>>;
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Default, Hash, Serialize, Deserialize, Error)]
#[error("{error}")]
pub struct ErrorResponse {
    pub error: String,
}

#[derive(Error, Debug)]
pub struct AppError<R = ErrorResponse> {
    #[source]
    source: Box<dyn std::error::Error>,
    status: StatusCode,
    response: R,
}
impl<R: std::error::Error + Serialize> IntoResponse for AppError<R> {
    fn into_response(self) -> Response {
        tracing::error!(
            source = %self.source,
            response = %self.response,
        );
        (self.status, Json(self.response)).into_response()
    }
}
pub trait IntoAppResult<T, E, R> {
    fn response(self, response: R) -> AppResult<T, E>;
}
impl<T, E: std::error::Error + 'static, R: Display + AsStatusCode> IntoAppResult<T, ErrorResponse, R> for Result<T, E> {
    fn response(self, response: R) -> AppResult<T, ErrorResponse> {
        self.map_err(|e| AppError {
            source: Box::new(e),
            status: response.status_code(),
            response: ErrorResponse { error: response.to_string() },
        })
    }
}

pub trait AsStatusCode {
    fn status_code(&self) -> StatusCode {
        APP_DEFAULT_ERROR_CODE
    }
}

#[cfg(test)]
mod tests {
    use crate::app::tests::body_bytes;

    use super::*;

    #[tokio::test]
    async fn test_error_response() {
        #[derive(Error, Debug)]
        enum Internal {
            #[error("error for server")]
            SomethingWentWrong,
        }
        #[derive(Error, Debug)]
        enum External {
            #[error("error for client")]
            SomethingWentWrong,
        }
        impl AsStatusCode for External {}
        let internal = Err::<(), _>(Internal::SomethingWentWrong);
        let external = internal.response(External::SomethingWentWrong);

        let external_err = external.unwrap_err();
        assert_eq!(external_err.source.to_string(), "error for server");
        assert_eq!(external_err.response.to_string(), "error for client");
        assert_eq!(external_err.status, APP_DEFAULT_ERROR_CODE);

        let expect = ErrorResponse { error: External::SomethingWentWrong.to_string() };
        assert_eq!(
            body_bytes(Json(expect).into_response().into_body()).await.unwrap(),
            body_bytes(external_err.into_response().into_body()).await.unwrap(),
        );
    }
}
