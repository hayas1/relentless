use example_http_server::serve;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let bind = "0.0.0.0:3000";
    serve(bind).await
}
