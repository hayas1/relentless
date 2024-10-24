use std::{
    fmt::{Debug, Display},
    marker::PhantomData,
};

use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use serde::{Deserialize, Serialize};
use thiserror::Error;

pub type AppError<K, T = ()> = AppErrorDetail<K, T>;

pub const APP_DEFAULT_ERROR_CODE: StatusCode = StatusCode::BAD_REQUEST;

#[derive(Error, Debug)]
#[error("{0}")]
pub struct Logged<T>(pub T);
#[derive(Error, Debug)]
pub struct AppErrorDetail<K, T> {
    #[source]
    pub source: Box<dyn std::error::Error + Send + Sync + 'static>,
    pub status: StatusCode,
    pub inner: AppErrorInner<K, T>,
}
impl<K: kind::Kind, T: Serialize> IntoResponse for AppErrorDetail<K, T> {
    fn into_response(self) -> Response {
        tracing::error!("cause error: {}", self.source); // TODO middleware (but route return Response<Bytes> that do not contain Err information)
        (self.status, self.inner).into_response()
    }
}
impl<K, T> AppErrorDetail<K, T> {
    pub fn new<E>(status: StatusCode, source: E, detail: T) -> Self
    where
        E: std::error::Error + Send + Sync + 'static,
    {
        let (source, msg) = (Box::new(source), PhantomData);
        Self { status, source, inner: AppErrorInner { msg, detail } }
    }

    pub fn detail<E>(source: E, detail: T) -> Self
    where
        E: std::error::Error + Send + Sync + 'static,
    {
        let status = APP_DEFAULT_ERROR_CODE;
        Self::new(status, source, detail)
    }

    pub fn inner(&self) -> &AppErrorInner<K, T> {
        &self.inner
    }
}
impl<K> AppErrorDetail<K, String> {
    pub fn detail_display<E>(source: E) -> Self
    where
        E: Display + std::error::Error + Send + Sync + 'static,
    {
        Self::detail_display_with_source(source, |e| e)
    }

    pub fn detail_display_with_source<E, S, F>(source: E, f: F) -> Self
    where
        E: Display,
        S: std::error::Error + Send + Sync + 'static,
        F: FnOnce(E) -> S,
    {
        let (status, msg) = (APP_DEFAULT_ERROR_CODE, source.to_string());
        Self::new(status, f(source), msg)
    }
}
impl<K> AppErrorDetail<K, ()> {
    pub fn wrap<E>(source: E) -> Self
    where
        E: std::error::Error + Send + Sync + 'static,
    {
        let status = APP_DEFAULT_ERROR_CODE;
        Self::new(status, source, ())
    }
}

pub mod kind {
    use super::*;

    // TODO attribute macro
    pub trait Kind {
        fn msg() -> &'static str;
    }

    #[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
    pub enum NotFound {}
    impl Kind for NotFound {
        fn msg() -> &'static str {
            "not found"
        }
    }

    #[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
    pub enum BadRequest {}
    impl Kind for BadRequest {
        fn msg() -> &'static str {
            "bad request"
        }
    }

    #[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
    pub enum Retriable {}
    impl Kind for Retriable {
        fn msg() -> &'static str {
            "please try again later"
        }
    }

    #[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
    pub enum Unreachable {}
    impl Kind for Unreachable {
        fn msg() -> &'static str {
            "something went wrong"
        }
    }
}
#[derive(Error, Debug, Clone, PartialEq, Eq)]
pub struct AppErrorInner<K, T> {
    pub msg: PhantomData<K>,
    pub detail: T,
}
impl<K: kind::Kind, T: Serialize> IntoResponse for AppErrorInner<K, T> {
    fn into_response(self) -> Response {
        Json(ErrorResponseInner::from(self)).into_response()
    }
}
#[derive(Error, Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ErrorResponseInner<T> {
    pub msg: String,
    pub detail: T,
}
impl<K: kind::Kind, T> From<AppErrorInner<K, T>> for ErrorResponseInner<T> {
    fn from(inner: AppErrorInner<K, T>) -> Self {
        Self { msg: K::msg().to_string(), detail: inner.detail }
    }
}

pub mod counter {
    use super::*;

    #[derive(Error, Debug, Clone, PartialEq, Eq)]
    pub enum CounterError<E> {
        #[error("overflow counter")]
        Overflow(E),

        #[error("cannot parse value as integer: {1}")]
        CannotParse(E, String),
    }

    impl<E> IntoResponse for CounterError<E>
    where
        E: std::error::Error + Send + Sync + 'static,
    {
        fn into_response(self) -> Response {
            AppErrorDetail::<kind::BadRequest, _>::detail_display_with_source(self, |e| match e {
                CounterError::Overflow(e) => e,
                CounterError::CannotParse(e, _) => e,
            })
            .into_response()
        }
    }
}

pub mod random {
    use crate::route::random::DistRangeParam;

    use super::*;

    #[derive(Error, Debug, Clone, PartialEq, Eq)]
    pub enum RandomError<T: Display> {
        #[error("`{0}` is empty range")]
        EmptyRange(DistRangeParam<T>),
    }

    impl<T: Display + Debug + Send + Sync + 'static> IntoResponse for RandomError<T> {
        fn into_response(self) -> Response {
            AppErrorDetail::<kind::BadRequest, _>::detail_display(self).into_response()
        }
    }
}
