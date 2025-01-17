use relentless_grpc_dev_server::{env::Env, serve};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let env = Env::environment();
    serve(env).await?;
    Ok(())
}
