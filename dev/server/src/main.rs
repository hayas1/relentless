use example_http_server::{env::Env, serve};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let env = Env::environment(Default::default());
    serve(env).await
}
