use std::sync::Arc;

use futures::lock::Mutex;

use crate::{env::Env, service::root::RootState};

#[derive(Clone, Default)]
pub struct AppState {
    pub env: Env,
    pub roots: Arc<Mutex<RootState>>,
}
