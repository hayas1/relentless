use crate::env::Env;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct State {
    pub env: Env,
}
