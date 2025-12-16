pub mod counter;
pub mod echo;
pub mod health;
pub mod information;
pub mod random;
pub mod root;
pub mod wait;

use std::{
    future::Future,
    pin::Pin,
    sync::{Arc, RwLock},
};

use axum::{
    http::{StatusCode, Uri},
    response::Result,
    routing::get,
    Router,
};
use axum_tracing_opentelemetry::middleware::{OtelAxumLayer, OtelInResponseLayer};
use tower::ServiceBuilder;
use tower_http::normalize_path::{NormalizePath, NormalizePathLayer};

use crate::{
    app::counter::CounterState,
    error::{kind::NotFound, AppErrorDetail, Logged},
    runner::RunCommand,
};

pub type PinResponseFuture<R> = Pin<Box<dyn Future<Output = R> + Send>>;

#[derive(Debug, Clone, Default)]
pub struct AppRouter {
    pub state: AppState,
}
impl AppRouter {
    pub fn service(self) -> NormalizePath<Router<()>> {
        ServiceBuilder::new()
            .layer(NormalizePathLayer::trim_trailing_slash())
            .service(self.router().layer(OtelInResponseLayer).layer(OtelAxumLayer::default()))
    }
    pub fn router(self) -> Router<()> {
        Router::new()
            .route("/", get(root::root))
            .nest("/health", health::route_health())
            .route("/healthz", get(health::health))
            .nest("/echo", echo::route_echo())
            .nest("/information", information::route_information())
            .nest("/counter", counter::route_counter())
            .nest("/wait", wait::route_wait())
            .nest("/random", random::route_random())
            .fallback(Self::not_found)
            .with_state(self.state)
    }

    pub async fn not_found(uri: Uri) -> Result<()> {
        Err(AppErrorDetail::<NotFound, _>::new(StatusCode::NOT_FOUND, Logged(""), uri.to_string()))?
    }
}
#[derive(Debug, Clone, Default)]
pub struct AppState {
    pub rc: Arc<RunCommand>,
    pub counter: Arc<RwLock<CounterState>>,
}

#[cfg(test)]
mod tests {

    use std::fmt::Debug;

    use axum::{
        body::{self, Body, Bytes, HttpBody},
        http::{Request, StatusCode},
        response::Response,
    };
    use serde::de::DeserializeOwned;
    use tower::Service;

    pub async fn call_bytes<S>(service: &mut S, req: Request<Body>) -> (StatusCode, Bytes)
    where
        S: Service<Request<Body>, Response = Response<Body>>,
        S::Error: Debug,
        Box<dyn std::error::Error + Send + Sync + 'static>: From<S::Error>,
    {
        let res = service.call(req).await.unwrap();
        let status = res.status();
        let size = res.size_hint().upper().unwrap_or(res.size_hint().lower()) as usize;
        let body = body::to_bytes(res.into_body(), size).await.unwrap();
        (status, body)
    }

    pub async fn call<S, T>(service: &mut S, req: Request<Body>) -> (StatusCode, T)
    where
        S: Service<Request<Body>, Response = Response<Body>>,
        S::Error: Debug,
        Box<dyn std::error::Error + Send + Sync + 'static>: From<S::Error>,
        T: DeserializeOwned,
    {
        let (status, body) = call_bytes(service, req).await;
        let des = serde_json::from_slice::<T>(&body).unwrap();
        (status, des)
    }

    pub async fn call_with_assert<S, T>(
        service: &mut S,
        req: Request<Body>,
        expected_status: StatusCode,
        expected_body: T,
    ) where
        S: Service<Request<Body>, Response = Response<Body>>,
        S::Error: Debug,
        Box<dyn std::error::Error + Send + Sync + 'static>: From<S::Error>,
        T: DeserializeOwned + Eq + std::fmt::Debug,
    {
        let (actual_status, actual_body): (_, T) = call(service, req).await;
        assert_eq!(actual_status, expected_status);
        assert_eq!(actual_body, expected_body);
    }

    pub async fn call_with_assert_ne_body<S, T>(
        service: &mut S,
        req: Request<Body>,
        expected_status: StatusCode,
        expected_body: T,
    ) where
        S: Service<Request<Body>, Response = Response<Body>>,
        S::Error: Debug,
        Box<dyn std::error::Error + Send + Sync + 'static>: From<S::Error>,
        T: DeserializeOwned + Eq + std::fmt::Debug,
    {
        let (actual_status, actual_body): (_, T) = call(service, req).await;
        assert_eq!(actual_status, expected_status);
        assert_ne!(actual_body, expected_body);
    }
}
