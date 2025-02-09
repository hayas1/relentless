use async_graphql::{Context, Object, Result, Schema, Subscription, ID};
use futures::{Stream, StreamExt};
use slab::Slab;

use crate::{simple_broker::SimpleBroker, state::AppState};

use super::{MutationRoot, MutationType, QueryRoot, SubscriptionRoot};

pub type ContentState = Slab<Content>;
pub type ContentSchema = Schema<QueryRoot, MutationRoot, SubscriptionRoot>;

#[derive(Clone)]
pub struct Content {
    id: ID,
    name: String,
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

#[Object]
impl QueryRoot {
    async fn contents(&self, ctx: &Context<'_>) -> Vec<Content> {
        let contents = ctx.data_unchecked::<AppState>().contents.lock().await;
        contents.iter().map(|(_, content)| content).cloned().collect()
    }
}

#[Object]
impl MutationRoot {
    async fn create_content(&self, ctx: &Context<'_>, name: String) -> ID {
        let mut contents = ctx.data_unchecked::<AppState>().contents.lock().await;
        let entry = contents.vacant_entry();
        let id: ID = entry.key().into();
        let content = Content { id: id.clone(), name };
        entry.insert(content);
        SimpleBroker::publish(ContentChanged { mutation_type: MutationType::Created, id: id.clone() });
        id
    }

    async fn delete_content(&self, ctx: &Context<'_>, id: ID) -> Result<bool> {
        let mut contents = ctx.data_unchecked::<AppState>().contents.lock().await;
        let id = id.parse::<usize>()?;
        if contents.contains(id) {
            contents.remove(id);
            SimpleBroker::publish(ContentChanged { mutation_type: MutationType::Deleted, id: id.into() });
            Ok(true)
        } else {
            Ok(false)
        }
    }
}

#[derive(Clone)]
struct ContentChanged {
    mutation_type: MutationType,
    id: ID,
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

#[Subscription]
impl SubscriptionRoot {
    async fn interval(&self, #[graphql(default = 1)] n: i32) -> impl Stream<Item = i32> {
        let mut value = 0;
        async_stream::stream! {
            loop {
                value += n;
                yield value;
            }
        }
    }

    async fn contents(&self, mutation_type: Option<MutationType>) -> impl Stream<Item = ContentChanged> {
        SimpleBroker::<ContentChanged>::subscribe().filter(move |event| {
            let res = if let Some(mutation_type) = mutation_type { event.mutation_type == mutation_type } else { true };
            async move { res }
        })
    }
}
