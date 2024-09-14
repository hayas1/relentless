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
        http::{Request, StatusCode},
    };
    use serde::de::DeserializeOwned;
    use tower::ServiceExt;

    use crate::{route, state::AppState};

    pub async fn oneshot_bytes(uri: &str, body: Body) -> (StatusCode, Bytes) {
        let app = route::app().with_state(AppState { env: Default::default() });
        let req = Request::builder().uri(uri).body(body).unwrap();
        let res = app.oneshot(req).await.unwrap();

        let size = res.size_hint().upper().unwrap_or(res.size_hint().lower()) as usize;
        (res.status(), to_bytes(res.into_body(), size).await.unwrap())
    }
    pub async fn oneshot<T: DeserializeOwned>(uri: &str, body: Body) -> (StatusCode, T) {
        let (status, bytes) = oneshot_bytes(uri, body).await;
        (status, serde_json::from_slice(&bytes).unwrap())
    }

    #[tokio::test]
    async fn test_root_call() {
        let (uri, body) = ("/", Body::empty());
        let (status, body) = oneshot_bytes(uri, body).await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(&body[..], b"Hello World");
    }
}
