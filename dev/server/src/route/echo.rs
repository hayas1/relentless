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
        .route("/json", get(jsonize))
        .route("/json", post(json_body))
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

#[tracing::instrument]
pub async fn jsonize() -> Json<Value> {
    todo!()
}

#[tracing::instrument]
pub async fn json_body(body: Json<Value>) -> Json<Value> {
    body
}

#[cfg(test)]
mod tests {
    use crate::route::app_with;
    use crate::route::tests::{call_bytes, call_with_assert};

    use super::*;
    use axum::body::Body;
    use axum::http::header::CONTENT_TYPE;
    use axum::http::{Method, Request, StatusCode};
    use mime::APPLICATION_JSON;

    #[tokio::test]
    async fn test_echo_empty_handler() {
        assert_eq!(empty().await, "");
    }

    #[tokio::test]
    async fn test_echo_empty() {
        let mut app = app_with(Default::default());

        let (status, body) = call_bytes(&mut app, Request::builder().uri("/echo/").body(Body::empty()).unwrap()).await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(&body[..], b"");
    }

    #[tokio::test]
    async fn test_echo_body() {
        let mut app = app_with(Default::default());

        let (status, body) = call_bytes(
            &mut app,
            Request::builder().uri("/echo/").method(Method::POST).body(Body::from("hello world")).unwrap(),
        )
        .await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(&body[..], b"hello world");
    }

    #[tokio::test]
    async fn test_echo_text() {
        let mut app = app_with(Default::default());

        let (status, body) =
            call_bytes(&mut app, Request::builder().uri("/echo/text/path?key=value").body(Body::empty()).unwrap())
                .await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(&body[..], b"/echo/text/path?key=value");
    }

    #[tokio::test]
    async fn test_echo_path() {
        let mut app = app_with(Default::default());

        let (status, body) =
            call_bytes(&mut app, Request::builder().uri("/echo/path/query?key=value").body(Body::empty()).unwrap())
                .await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(&body[..], b"query");
    }

    #[tokio::test]
    async fn test_echo_method() {
        let mut app = app_with(Default::default());

        let (status, body) = call_bytes(
            &mut app,
            Request::builder().uri("/echo/method").method(Method::OPTIONS).body(Body::empty()).unwrap(),
        )
        .await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(&body[..], b"OPTIONS");
    }

    #[tokio::test]
    async fn test_echo_headers() {
        let mut app = app_with(Default::default());

        call_with_assert(
            &mut app,
            Request::builder()
                .uri("/echo/headers")
                .header("key1", "value1")
                .header("key2", "value2")
                .body(Body::empty())
                .unwrap(),
            StatusCode::OK,
            json!([{ "key1": "value1" }, { "key2": "value2" }]),
        )
        .await;
    }

    #[tokio::test]
    async fn test_echo_json() {
        // let mut app = app_with(Default::default());

        // call_with_assert(
        //     &mut app,
        //     Request::builder().uri("/echo/json").body(Body::empty()).unwrap(),
        //     StatusCode::OK,
        //     json!({}),
        // )
        // .await;
    }

    #[tokio::test]
    async fn test_echo_json_post() {
        let mut app = app_with(Default::default());

        call_with_assert(
            &mut app,
            Request::builder()
                .uri("/echo/json")
                .method(Method::POST)
                .header(CONTENT_TYPE, APPLICATION_JSON.as_ref())
                .body(Body::from(r#"{"key": "value"}"#))
                .unwrap(),
            StatusCode::OK,
            json!({ "key": "value" }),
        )
        .await;
    }
}
