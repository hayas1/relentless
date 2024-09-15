use example_http_server::serve;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let env = Default::default();
    serve(env).await
}
