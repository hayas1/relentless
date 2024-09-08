use axum::routing::get;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    #[tracing::instrument]
    async fn root_handler() -> String {
        "Hello World".to_string()
    }

    tracing_subscriber::fmt::init();
    let app = axum::Router::new()
        .route("/", get(root_handler))
        .layer(axum::middleware::from_fn(log_request_response));

    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await?;
    Ok(axum::serve(listener, app).await?)
}

pub async fn log_request_response(
    req: axum::http::Request<axum::body::Body>,
    next: axum::middleware::Next,
    // ) -> Result<impl axum::response::IntoResponse, (axum::http::StatusCode, String)> {
) -> impl axum::response::IntoResponse {
    let (method, uri) = (req.method().clone(), req.uri().clone());
    let res = next.run(req).await;
    let status = res.status();
    tracing::info!("{} {} {}", status, method, uri);
    res
}
