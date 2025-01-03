pub mod env;
pub mod middleware;
pub mod service;

pub async fn serve(env: env::Env) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    tracing_subscriber::fmt::init();
    let addr = env.bind().parse()?;
    let server = service::app(env).await;

    tracing::info!("start app on {}", addr);
    server
        .serve_with_shutdown(addr, async {
            tokio::signal::ctrl_c().await.expect("failed to install Ctrl+C handler");
        })
        .await?;
    tracing::info!("stop app");

    Ok(())
}
