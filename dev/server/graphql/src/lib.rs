pub mod book;
pub mod route;
pub mod simple_broker;

pub async fn serve() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:8000").await?;
    let app = route::app();

    axum::serve(listener, app).await?;
    Ok(())
}
