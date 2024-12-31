use relentless_dev_server_http::{env::Env, serve};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let env = Env::environment(Default::default());
    serve(env).await
}
