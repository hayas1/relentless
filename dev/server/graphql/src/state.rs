use std::sync::Arc;

use futures::lock::Mutex;

use crate::{book::BookState, env::Env};

#[derive(Clone, Default)]
pub struct AppState {
    pub env: Env,
    pub books: Arc<Mutex<BookState>>,
}
