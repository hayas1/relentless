pub mod counter;
pub mod echo;
pub mod health;
pub mod information;
pub mod random;
pub mod root;
pub mod wait;

use std::{future::Future, net::SocketAddr, pin::Pin};

use axum::{
    body::{Body, HttpBody},
    extract::{connect_info::IntoMakeServiceWithConnectInfo, Request},
    http::{StatusCode, Uri},
    middleware::{self, Next},
    response::{IntoResponse, Result},
    routing::get,
    Router,
};
use tower::Layer;
use tower_http::normalize_path::{NormalizePath, NormalizePathLayer};

use crate::{
    env::Env,
    error::{kind::NotFound, AppErrorDetail, Logged},
    state::AppState,
};

pub type PinResponseFuture<R> = Pin<Box<dyn Future<Output = R> + Send>>;

pub fn app_with(env: Env) -> NormalizePath<Router<()>> {
    let state = AppState { env, ..Default::default() };
    app(state)
}
pub fn app(state: AppState) -> NormalizePath<Router<()>> {
    NormalizePathLayer::trim_trailing_slash().layer(router(state))
}
pub fn router(state: AppState) -> Router<()> {
    Router::new()
        .route("/", get(root::root))
        .nest("/health", health::route_health())
        .route("/healthz", get(health::health))
        .nest("/echo", echo::route_echo())
        .nest("/information", information::route_information())
        .nest("/counter", counter::route_counter())
        .nest("/wait", wait::route_wait())
        .nest("/random", random::route_random())
        .fallback(not_found)
        .layer(middleware::from_fn_with_state(state.clone(), logging))
        .with_state(state)
}
pub fn router_with_connect_info(state: AppState) -> IntoMakeServiceWithConnectInfo<Router<()>, SocketAddr> {
    // TODO how to use this with NormalizePathLayer ?
    router(state).into_make_service_with_connect_info()
}

pub async fn not_found(uri: Uri) -> Result<()> {
    Err(AppErrorDetail::<NotFound, _>::new(StatusCode::NOT_FOUND, Logged(""), uri.to_string()))?
}

pub async fn logging(req: Request<Body>, next: Next) -> impl IntoResponse {
    let (method, uri) = (req.method().clone(), req.uri().clone());
    let res = next.run(req).await;
    let (status, bytes) = (res.status(), res.size_hint().lower());
    tracing::info!("{} {} {} {}", status, method, uri, bytes);
    res
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

    pub async fn call_bytes<S>(app: &mut S, req: Request<Body>) -> (StatusCode, Bytes)
    where
        S: Service<Request<Body>, Response = Response<Body>>,
        S::Error: Debug,
        Box<dyn std::error::Error + Send + Sync + 'static>: From<S::Error>,
    {
        let res = app.call(req).await.unwrap();
        let status = res.status();
        let size = res.size_hint().upper().unwrap_or(res.size_hint().lower()) as usize;
        let body = body::to_bytes(res.into_body(), size).await.unwrap();
        (status, body)
    }

    pub async fn call<S, T>(app: &mut S, req: Request<Body>) -> (StatusCode, T)
    where
        S: Service<Request<Body>, Response = Response<Body>>,
        S::Error: Debug,
        Box<dyn std::error::Error + Send + Sync + 'static>: From<S::Error>,
        T: DeserializeOwned,
    {
        let (status, body) = call_bytes(app, req).await;
        let des = serde_json::from_slice::<T>(&body).unwrap();
        (status, des)
    }

    pub async fn call_with_assert<S, T>(app: &mut S, req: Request<Body>, expected_status: StatusCode, expected_body: T)
    where
        S: Service<Request<Body>, Response = Response<Body>>,
        S::Error: Debug,
        Box<dyn std::error::Error + Send + Sync + 'static>: From<S::Error>,
        T: DeserializeOwned + Eq + std::fmt::Debug,
    {
        let (actual_status, actual_body): (_, T) = call(app, req).await;
        assert_eq!(actual_status, expected_status);
        assert_eq!(actual_body, expected_body);
    }

    pub async fn call_with_assert_ne_body<S, T>(
        app: &mut S,
        req: Request<Body>,
        expected_status: StatusCode,
        expected_body: T,
    ) where
        S: Service<Request<Body>, Response = Response<Body>>,
        S::Error: Debug,
        Box<dyn std::error::Error + Send + Sync + 'static>: From<S::Error>,
        T: DeserializeOwned + Eq + std::fmt::Debug,
    {
        let (actual_status, actual_body): (_, T) = call(app, req).await;
        assert_eq!(actual_status, expected_status);
        assert_ne!(actual_body, expected_body);
    }
}
