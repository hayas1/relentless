use axum::{
    body::{to_bytes, Body, Bytes, HttpBody},
    http::{HeaderMap, Request, StatusCode},
};
use serde::de::DeserializeOwned;
use tower::ServiceExt;

use example_http_server::{
    route::{self, health::Health},
    state::AppState,
};

pub async fn send_bytes(
    uri: &str,
    body: Body,
    headers: HeaderMap,
) -> Result<(StatusCode, Bytes), Box<dyn std::error::Error>> {
    let state = AppState { env: Default::default() };
    let app = route::app(state);
    let mut req = Request::builder().uri(uri).body(body)?;
    for (key, val) in headers {
        req.headers_mut().insert(key.ok_or("no key")?, val);
    }
    let res = app.oneshot(req).await?;

    let size = res.size_hint().upper().unwrap_or(res.size_hint().lower()) as usize;
    Ok((res.status(), to_bytes(res.into_body(), size).await?))
}
pub async fn send<T: DeserializeOwned>(
    uri: &str,
    body: Body,
    headers: HeaderMap,
) -> Result<(StatusCode, T), Box<dyn std::error::Error>> {
    let (status, bytes) = send_bytes(uri, body, headers).await?;
    Ok((status, serde_json::from_slice(&bytes)?))
}

#[tokio::test]
async fn test_root_call() {
    let (uri, body, headers) = ("/", Body::empty(), HeaderMap::new());
    let (status, body) = send_bytes(uri, body, headers).await.unwrap();
    assert_eq!(status, StatusCode::OK);
    assert_eq!(&body[..], b"Hello World");
}

#[tokio::test]
async fn test_healthz_call() {
    let (uri, body, headers) = ("/healthz", Body::empty(), HeaderMap::new());
    let (status, body) = send_bytes(uri, body, headers).await.unwrap();
    assert_eq!(status, StatusCode::OK);
    assert_eq!(&body[..], b"ok");
}

#[tokio::test]
async fn test_health_call() {
    let (uri, body, headers) = ("/health", Body::empty(), HeaderMap::new());
    let (status, body) = send_bytes(uri, body, headers).await.unwrap();
    assert_eq!(status, StatusCode::OK);
    assert_eq!(&body[..], b"ok");
}

#[tokio::test]
async fn test_health_rich_call() {
    let (uri, body, headers) = ("/health/rich", Body::empty(), HeaderMap::new());
    let (status, health) = send::<Health>(uri, body, headers).await.unwrap();
    assert_eq!(status, StatusCode::OK);
    assert_eq!(health, Health { status: StatusCode::OK });
}

#[tokio::test]
async fn test_disabled_call() {
    let (uri, body, headers) = ("/health/disabled", Body::empty(), HeaderMap::new());
    let (status, health) = send::<Health>(uri, body, headers).await.unwrap();
    assert_eq!(status, StatusCode::SERVICE_UNAVAILABLE);
    assert_eq!(health, Health { status: StatusCode::SERVICE_UNAVAILABLE });
}
