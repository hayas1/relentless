use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
};
use thiserror::Error;

pub type AppResult<T, R = ()> = Result<T, AppError<R>>;

pub const APP_DEFAULT_ERROR_CODE: StatusCode = StatusCode::BAD_REQUEST;

#[derive(Error, Debug)]
#[error(transparent)]
pub enum AppError<R = ()> {
    Response(#[from] ResponseWithError<R>),

    BoxError(#[from] Box<dyn std::error::Error + Send + Sync + 'static>),
}

impl<R: IntoResponse + std::fmt::Debug> IntoResponse for AppError<R> {
    fn into_response(self) -> Response {
        tracing::error!("error: {:?}", self); // TODO middleware
        match self {
            AppError::Response(response) => response.into_response(),
            _ => (APP_DEFAULT_ERROR_CODE, "bad request").into_response(),
        }
    }
}

#[derive(Error, Debug)]
pub struct ResponseWithError<R> {
    status: StatusCode,
    response: R,
}
impl<R: IntoResponse> IntoResponse for ResponseWithError<R> {
    fn into_response(self) -> Response {
        (self.status, self.response).into_response()
    }
}
impl<R: IntoResponse> From<(StatusCode, R)> for ResponseWithError<R> {
    fn from((status, response): (StatusCode, R)) -> Self {
        Self::new(status, response)
    }
}
impl<R: IntoResponse> From<R> for ResponseWithError<R> {
    fn from(response: R) -> Self {
        Self::new(APP_DEFAULT_ERROR_CODE, response)
    }
}
impl<R: IntoResponse> ResponseWithError<R> {
    pub fn new(status: StatusCode, response: R) -> Self {
        Self { status, response }
    }
}
