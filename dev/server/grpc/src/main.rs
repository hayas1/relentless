use relentless_dev_server_grpc::{env, serve};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    serve(env::Env::environment(Default::default())).await?;
    Ok(())
}
