pub mod env;
pub mod service;
pub mod simple_broker;
pub mod state;

pub async fn serve(env: env::Env) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    tracing_subscriber::fmt::init();
    let listener = tokio::net::TcpListener::bind(&env.bind()).await?;
    let app = service::app(env);

    tracing::info!("start app on {}", listener.local_addr()?);
    axum::serve(listener, app)
        .with_graceful_shutdown(async {
            tokio::signal::ctrl_c().await.expect("failed to install Ctrl+C handler");
        })
        .await?;
    tracing::info!("stop app");
    Ok(())
}
