use relentless_graphql_dev_server::serve;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    serve().await
}
