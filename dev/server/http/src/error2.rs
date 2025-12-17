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
        tracing::error!( // TODO middleware (but route return Response<Bytes> that do not contain Err information)
            source = %self.source,
            response = %self.response,
        );
        (self.status, Json(self.response)).into_response()
    }
}
pub trait IntoAppResult<T, E, R> {
    fn response(self, response: R) -> AppResult<T, E>;
    // TODO fn map_response<F: FnOnce(&E) -> R>(self, f: F) -> AppResult<T, E>;
    fn response_code(self, response: R, status: StatusCode) -> AppResult<T, E>;
}
impl<T, E: std::error::Error + 'static, R: Display> IntoAppResult<T, ErrorResponse, R> for Result<T, E> {
    fn response(self, response: R) -> AppResult<T, ErrorResponse> {
        self.response_code(response, APP_DEFAULT_ERROR_CODE)
    }
    fn response_code(self, response: R, status: StatusCode) -> AppResult<T, ErrorResponse> {
        self.map_err(|e| AppError {
            source: Box::new(e),
            status,
            response: ErrorResponse { error: response.to_string() },
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_response() {
        #[derive(Error, Debug)]
        enum Internal {
            #[error("error for server")]
            SomethingWentWrong,
        }
        let internal = Err::<(), _>(Internal::SomethingWentWrong);
        let external = internal.response("error for client");

        let external_err = external.unwrap_err();
        assert_eq!(external_err.source.to_string(), "error for server");
        assert_eq!(external_err.response.to_string(), "error for client");
        assert_eq!(external_err.status, APP_DEFAULT_ERROR_CODE);
    }
}
