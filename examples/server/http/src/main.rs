use example_http_server::{env, serve};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let env = env::Env { bind: "0.0.0.0:3000".to_string() };
    serve(env).await
}
