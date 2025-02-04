use std::sync::Arc;

use futures::lock::Mutex;

use crate::{env::Env, service::book::BookState};

#[derive(Clone, Default)]
pub struct AppState {
    pub env: Env,
    pub books: Arc<Mutex<BookState>>,
}
