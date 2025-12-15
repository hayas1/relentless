use std::sync::{Arc, RwLock};

use crate::{route::counter::CounterState, runner::RunCommand};

#[derive(Debug, Clone, Default)]
pub struct AppState {
    pub env: Arc<RunCommand>,
    pub counter: Arc<RwLock<CounterState>>,
}
