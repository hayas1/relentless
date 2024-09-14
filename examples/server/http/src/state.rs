use crate::env::Env;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AppState {
    pub env: Env,
}
