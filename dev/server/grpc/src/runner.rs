use std::sync::Arc;

use clap::Parser;

use crate::app::{AppState, Application};

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
        let router = self.app().router().await;

        tracing::info!("start app on {}", addr);
        router
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
    pub fn app(&self) -> Application {
        let state = AppState { rc: Arc::new(self.clone()), counter: Default::default() };
        Application { state }
    }
}
