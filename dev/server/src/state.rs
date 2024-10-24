use std::sync::{Arc, RwLock};

use crate::{env::Env, route::counter::CounterState};

#[derive(Debug, Clone, Default)]
pub struct AppState {
    pub env: Env,
    pub counter: Arc<RwLock<CounterState>>,
}
