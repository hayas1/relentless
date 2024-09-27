use std::fmt::{Debug, Display};

use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use serde::{Deserialize, Serialize};
use thiserror::Error;

pub type AppResult<T, R = Json<ErrorMessageResponse<()>>> = Result<T, AppError<R>>;

pub const APP_DEFAULT_ERROR_CODE: StatusCode = StatusCode::BAD_REQUEST;

#[derive(Error, Debug)]
#[error(transparent)]
pub enum AppError<R> {
    Response(#[from] ResponseWithError<R>), // TODO should be always R = Json<ErrorMessageResponse<()>> here ?

    Counter(#[from] counter::CounterError),

    Anyhow(#[from] anyhow::Error),
    BoxError(#[from] Box<dyn std::error::Error + Send + Sync + 'static>),
}

impl<R: IntoResponse + Debug> IntoResponse for AppError<R> {
    fn into_response(self) -> Response {
        tracing::error!("error: {:?}", self); // TODO middleware
        match self {
            AppError::Response(response) => response.into_response(),
            AppError::Counter(c) => c.into_response(),
            AppError::Anyhow(_) | AppError::BoxError(_) => ResponseWithError::default().into_response(),
        }
    }
}

#[derive(Error, Debug)]
pub struct ResponseWithError<R> {
    status: StatusCode,
    response: R,
}
impl Default for ResponseWithError<Json<ErrorMessageResponse<()>>> {
    fn default() -> Self {
        Self::new(APP_DEFAULT_ERROR_CODE, Json(ErrorMessageResponse::default()))
    }
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
impl<R> ResponseWithError<R> {
    pub fn new(status: StatusCode, response: R) -> Self {
        Self { status, response }
    }
}

#[derive(Error, Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ErrorMessageResponse<T> {
    msg: String,
    detail: T,
}
impl Default for ErrorMessageResponse<()> {
    fn default() -> Self {
        Self::new("bad request".to_string(), ())
    }
}
impl<T: IntoResponse> From<T> for ErrorMessageResponse<()>
where
    T: std::error::Error + Send + Sync + 'static,
{
    fn from(e: T) -> Self {
        let log = e.to_string();
        tracing::error!("error: {:?}", log); // TODO middleware
        Default::default()
    }
}
impl<T> ErrorMessageResponse<T> {
    pub fn new(msg: String, detail: T) -> Self {
        Self { msg, detail }
    }
}
impl<T> ErrorMessageResponse<T> {
    pub fn detail<M: Display>(msg: M, detail: T) -> Self {
        Self::new(msg.to_string(), detail)
    }
}
impl ErrorMessageResponse<()> {
    pub fn msg<M: Display>(msg: M) -> Self {
        Self::new(msg.to_string(), ())
    }
}

pub mod counter {
    use super::*;

    #[derive(Error, Debug)]
    pub enum CounterError {
        #[error("overflow counter")]
        Overflow,

        #[error("cannot parse value as integer")]
        CannotParse(String),

        #[error("please try again later")]
        Retriable, // TODO AppError

        #[error("something went wrong")]
        Unreachable, // TODO AppError
    }

    impl IntoResponse for CounterError {
        fn into_response(self) -> Response {
            let msg = self.to_string();
            match self {
                Self::CannotParse(s) => {
                    (APP_DEFAULT_ERROR_CODE, Json(ErrorMessageResponse::detail(msg, s))).into_response()
                }
                _ => (APP_DEFAULT_ERROR_CODE, Json(ErrorMessageResponse::msg(msg))).into_response(),
            }
        }
    }
}
