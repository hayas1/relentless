use std::sync::Arc;

use clap::Parser;

use crate::app::{AppRouter, AppState};

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Default, Hash, Parser)]
pub struct RunCommand {
    /// Server listen
    #[arg(env, long, default_value = "0.0.0.0")]
    pub listen: String,

    /// Server port
    #[arg(env, long, default_value = "50051")]
    pub port: String,
}
impl RunCommand {
    pub fn cli() -> Self {
        <Self as Parser>::parse()
    }
    pub async fn serve(self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let addr = self.bind().parse()?;
        let service = self.app().service().await;

        tracing::info!("start app on {}", addr);
        service
            .serve_with_shutdown(addr, async {
                tokio::signal::ctrl_c().await.expect("failed to install Ctrl+C handler");
            })
            .await?;
        tracing::info!("stop app");

        Ok(())
    }

    pub fn bind(&self) -> String {
        format!("{}:{}", self.listen, self.port)
    }
    pub fn app(&self) -> AppRouter {
        let state = AppState { rc: Arc::new(self.clone()), counter: Default::default() };
        AppRouter { state }
    }
}
