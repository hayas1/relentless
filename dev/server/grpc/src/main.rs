use relentless_dev_server_grpc::{env::Env, serve};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let env = Env::environment();
    serve(env).await?;
    Ok(())
}
