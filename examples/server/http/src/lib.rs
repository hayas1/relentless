pub mod env;
pub mod error;
pub mod route;
pub mod state;

pub async fn serve(env: env::Env) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    tracing_subscriber::fmt::init();
    let listener = tokio::net::TcpListener::bind(&env.bind()).await?;
    let app = route::app(env);
    tracing::info!("start app on {}", listener.local_addr()?);
    let serve = axum::serve(listener, app).with_graceful_shutdown(async {
        tokio::signal::ctrl_c().await.expect("failed to install Ctrl+C handler");
        tracing::info!("stop app");
    });
    Ok(serve.await?)
}
