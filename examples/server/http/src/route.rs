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

use crate::state::AppState;

pub fn app(state: AppState) -> Router<()> {
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
