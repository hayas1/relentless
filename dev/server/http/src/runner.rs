use std::sync::{Arc, RwLock};

use axum::{extract::Request, ServiceExt};
use clap::Parser;
use tokio::net::TcpListener;

use crate::{
    route::{counter::CounterState, AppRouter},
    state::AppState,
};

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Default, Hash, Parser)]
pub struct RunCommand {
    /// Server listen
    #[arg(env, long, default_value = "0.0.0.0")]
    pub listen: String,

    /// Server port
    #[arg(env, long, default_value = "3000")]
    pub port: String,
}
impl RunCommand {
    pub fn cli() -> Self {
        <Self as Parser>::parse()
    }
    pub async fn serve(self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let listener = TcpListener::bind(&self.bind()).await?;
        let app = ServiceExt::<Request>::into_make_service(self.app().service());
        tracing::info!("start app on {}", listener.local_addr()?);
        axum::serve(listener, app)
            .with_graceful_shutdown(async {
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
        let state = AppState { env: Arc::new(self.clone()), counter: Arc::new(RwLock::new(CounterState::default())) };
        AppRouter { state }
    }
}
