pub mod env;
pub mod error;
pub mod route;
pub mod state;

pub async fn serve(env: env::Env) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    tracing_subscriber::fmt::init();
    let listener = tokio::net::TcpListener::bind(&env.bind).await?;
    let app = route::app().with_state(state::AppState { env });
    tracing::info!("start app on {}", listener.local_addr()?);
    let serve = axum::serve(listener, app).with_graceful_shutdown(async {
        tokio::signal::ctrl_c().await.expect("failed to install Ctrl+C handler");
        tracing::info!("stop app");
    });
    Ok(serve.await?)
}

#[cfg(test)]
mod tests {
    use axum::{
        body::{to_bytes, Body, Bytes, HttpBody},
        http::{HeaderMap, Request, StatusCode},
    };
    use serde::de::DeserializeOwned;
    use tower::ServiceExt;

    use crate::{route, state::AppState};

    pub async fn send_bytes(uri: &str, body: Body, headers: HeaderMap) -> (StatusCode, Bytes) {
        let app = route::app().with_state(AppState { env: Default::default() });
        let mut req = Request::builder().uri(uri).body(body).unwrap();
        for (key, val) in headers {
            req.headers_mut().insert(key.unwrap(), val);
        }
        let res = app.oneshot(req).await.unwrap();

        let size = res.size_hint().upper().unwrap_or(res.size_hint().lower()) as usize;
        (res.status(), to_bytes(res.into_body(), size).await.unwrap())
    }
    pub async fn send<T: DeserializeOwned>(uri: &str, body: Body, headers: HeaderMap) -> (StatusCode, T) {
        let (status, bytes) = send_bytes(uri, body, headers).await;
        (status, serde_json::from_slice(&bytes).unwrap())
    }

    #[tokio::test]
    async fn test_root_call() {
        let (uri, body, headers) = ("/", Body::empty(), HeaderMap::new());
        let (status, body) = send_bytes(uri, body, headers).await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(&body[..], b"Hello World");
    }
}
