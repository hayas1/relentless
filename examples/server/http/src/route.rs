pub mod counter;
pub mod health;
pub mod root;

use axum::{
    body::{Body, HttpBody},
    http::Request,
    middleware::{self, Next},
    response::IntoResponse,
    routing::get,
    Router,
};

use crate::{env::Env, state::AppState};

pub fn app(env: Env) -> Router<()> {
    let state = AppState { env, ..Default::default() };

    Router::new()
        .route("/", get(root::root))
        .nest("/health", health::route_health())
        .route("/healthz", get(health::health))
        .nest("/counter", counter::route_counter())
        .layer(middleware::from_fn_with_state(state.clone(), logging))
        .with_state(state)
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
        body::{self, Body, HttpBody},
        http::{Request, StatusCode},
        response::Response,
    };
    use serde::de::DeserializeOwned;
    use tower::Service;

    pub async fn call<S, T>(app: &mut S, req: Request<Body>) -> (StatusCode, T)
    where
        S: Service<Request<Body>, Response = Response<Body>>,
        S::Error: Debug,
        Box<dyn std::error::Error + Send + Sync + 'static>: From<S::Error>,
        T: DeserializeOwned,
    {
        let res = app.call(req).await.unwrap();
        let status = res.status();
        let size = res.size_hint().upper().unwrap_or(res.size_hint().lower()) as usize;
        let body = body::to_bytes(res.into_body(), size).await.unwrap();
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
}
