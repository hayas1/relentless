use axum::{
    body::Bytes,
    extract::{OriginalUri, Path, Request},
    http::HeaderMap,
    routing::{any, get, post},
    Json, Router,
};
use serde_json::{json, Value};

use crate::state::AppState;

pub fn route_echo() -> Router<AppState> {
    Router::new()
        .route("/", get(empty))
        .route("/", post(body))
        .route("/text/*rest", any(text))
        .route("/path/*rest", any(path))
        .route("/method", any(method))
        .route("/headers", any(headers))
}

#[tracing::instrument]
pub async fn empty() -> &'static str {
    ""
}

#[tracing::instrument]
pub async fn body(body: Bytes) -> Bytes {
    body
}

#[tracing::instrument]
pub async fn text(OriginalUri(uri): OriginalUri) -> String {
    uri.to_string()
}

#[tracing::instrument]
pub async fn path(Path(rest): Path<String>) -> String {
    rest
}

#[tracing::instrument]
pub async fn method(request: Request) -> String {
    request.method().to_string()
}

#[tracing::instrument]
pub async fn headers(headers: HeaderMap) -> Json<Value> {
    Json(
        headers
            .into_iter()
            .map(|(name, value)| {
                let v = String::from_utf8_lossy(value.as_bytes()).to_string();
                if let Some(n) = name.as_ref().map(ToString::to_string) {
                    json!({n: v})
                } else {
                    json!(v)
                }
            })
            .collect(),
    )
}
