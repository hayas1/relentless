use axum::routing::get;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    async fn root_handler() -> String {
        "Hello World".to_string()
    }

    let app = axum::Router::new().route("/", get(root_handler));

    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await?;
    Ok(axum::serve(listener, app).await?)
}
