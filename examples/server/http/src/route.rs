pub mod health;
pub mod root;

use axum::{body::HttpBody, middleware, routing::get};

use crate::state::AppState;

pub fn app() -> axum::Router<AppState> {
    axum::Router::new()
        .route("/", get(root::root))
        .nest("/health", health::route_health())
        .route("/healthz", get(health::health))
        .layer(middleware::from_fn(logging))
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

#[cfg(test)]
mod tests {
    use axum::{
        body::Body,
        http::{HeaderMap, StatusCode},
    };

    use crate::tests::send_bytes;

    #[tokio::test]
    async fn test_healthz_call() {
        let (uri, body, headers) = ("/healthz", Body::empty(), HeaderMap::new());
        let (status, body) = send_bytes(uri, body, headers).await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(&body[..], b"ok");
    }
}
