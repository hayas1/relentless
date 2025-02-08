use std::sync::Arc;

use futures::lock::Mutex;

use crate::{env::Env, service::root::ContentState};

#[derive(Clone, Default)]
pub struct AppState {
    pub env: Env,
    pub contents: Arc<Mutex<ContentState>>,
}
