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
        body::{to_bytes, Body},
        http::{Request, StatusCode},
    };
    use tower::ServiceExt;

    use super::*;
    use crate::state::AppState;

    #[tokio::test]
    async fn test_root_call() {
        let (uri, body) = ("/", Body::empty());

        let api = app().with_state(AppState { env: Default::default() });
        let response = api.oneshot(Request::builder().uri(uri).body(body).unwrap()).await.unwrap();

        assert_eq!(response.status(), StatusCode::OK);
        let size = response.size_hint().upper().unwrap_or(response.size_hint().lower()) as usize;
        let body = to_bytes(response.into_body(), size).await.unwrap();
        assert_eq!(&body[..], b"Hello World");
    }

    #[tokio::test]
    async fn test_health_call() {
        let (uri, body) = ("/health", Body::empty());

        let api = app().with_state(AppState { env: Default::default() });
        let response = api.oneshot(Request::builder().uri(uri).body(body).unwrap()).await.unwrap();

        assert_eq!(response.status(), StatusCode::OK);
        let size = response.size_hint().upper().unwrap_or(response.size_hint().lower()) as usize;
        let body = to_bytes(response.into_body(), size).await.unwrap();
        assert_eq!(&body[..], b"ok");
    }
}
