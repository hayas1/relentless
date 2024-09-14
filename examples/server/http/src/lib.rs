use axum::{body::HttpBody, routing::get};
use tokio::net::ToSocketAddrs;

pub async fn serve<A>(bind: A) -> Result<(), Box<dyn std::error::Error + Send + Sync>>
where
    A: ToSocketAddrs,
{
    tracing_subscriber::fmt::init();
    let app = route();
    let listener = tokio::net::TcpListener::bind(bind).await?;
    tracing::info!("start app on {}", listener.local_addr()?);
    let serve = axum::serve(listener, app).with_graceful_shutdown(async {
        tokio::signal::ctrl_c().await.expect("failed to install Ctrl+C handler");
        tracing::info!("stop app");
    });
    Ok(serve.await?)
}

pub fn route() -> axum::Router {
    axum::Router::new().route("/", get(root_handler)).layer(axum::middleware::from_fn(logging))
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

#[tracing::instrument]
async fn root_handler() -> String {
    "Hello World".to_string()
}
