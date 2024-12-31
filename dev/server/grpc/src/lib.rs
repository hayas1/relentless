use service::app_with;

pub mod env;
pub mod service;
pub mod state;

pub async fn serve(env: env::Env) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let addr = env.bind().parse()?;
    let server = app_with(env);
    server.serve(addr).await?;

    Ok(())
}
