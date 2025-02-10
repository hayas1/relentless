use async_graphql::{Context, Object, Result, Schema, ID};
use slab::Slab;

use crate::state::AppState;

use super::{MutationRoot, MutationType, QueryRoot, SubscriptionRoot};

pub type ContentState = Slab<Content>;
pub type ContentSchema = Schema<QueryRoot, MutationRoot, SubscriptionRoot>;

#[derive(Clone)]
pub struct Content {
    pub id: ID,
    pub name: String,
}

#[Object]
impl Content {
    async fn id(&self) -> &str {
        &self.id
    }

    async fn name(&self) -> &str {
        &self.name
    }
}

#[derive(Clone)]
pub struct ContentChanged {
    pub mutation_type: MutationType,
    pub id: ID,
}

#[Object]
impl ContentChanged {
    async fn mutation_type(&self) -> MutationType {
        self.mutation_type
    }

    async fn id(&self) -> &ID {
        &self.id
    }

    async fn content(&self, ctx: &Context<'_>) -> Result<Option<Content>> {
        let contents = ctx.data_unchecked::<AppState>().contents.lock().await;
        let id = self.id.parse::<usize>()?;
        Ok(contents.get(id).cloned())
    }
}
