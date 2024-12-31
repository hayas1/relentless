pub mod env;
pub mod service;

pub async fn serve(env: env::Env) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    tracing_subscriber::fmt::init();
    let addr = env.bind().parse()?;
    let server = service::app_with(env);

    tracing::info!("start app on {}", addr);
    server.serve(addr).await?;
    tracing::info!("stop app");

    Ok(())
}
