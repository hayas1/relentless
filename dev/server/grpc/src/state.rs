use crate::env::Env;

#[derive(Debug, Clone, Default)]
pub struct AppState {
    pub env: Env,
}
