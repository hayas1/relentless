use std::sync::{Arc, RwLock};

use crate::{env::Env, route::counter::Counter};

#[derive(Debug, Clone, Default)]
pub struct AppState {
    pub env: Env,
    pub counter: Arc<RwLock<Counter>>,
}
