pub mod root;

use axum::{body::HttpBody, middleware, routing::get};

use crate::state::State;

pub fn app() -> axum::Router<State> {
    axum::Router::new().route("/", get(root::root)).layer(middleware::from_fn(logging))
}

pub async fn logging(
    req: axum::http::Request<axum::body::Body>,
    next: axum::middleware::Next,
    // ) -> Result<impl axum::response::IntoResponse, (axum::http::StatusCode, String)> {
) -> impl axum::response::IntoResponse {
    let (method, uri) = (req.method().clone(), req.uri().clone());
    let res = next.run(req).await;
    let (status, bytes) = (res.status(), res.size_hint().lower());
    tracing::info!("{} {} {} {}", status, method, uri, bytes);
    res
}
