use async_graphql::{Context, Object, Result, Schema, Subscription, ID};
use futures::{Stream, StreamExt};
use slab::Slab;

use crate::{simple_broker::SimpleBroker, state::AppState};

use super::{MutationRoot, MutationType, QueryRoot, SubscriptionRoot};

pub type RootState = Slab<Root>;
pub type RootSchema = Schema<QueryRoot, MutationRoot, SubscriptionRoot>;

#[derive(Clone)]
pub struct Root {
    id: ID,
    name: String,
}

#[Object]
impl Root {
    async fn id(&self) -> &str {
        &self.id
    }

    async fn name(&self) -> &str {
        &self.name
    }
}

#[Object]
impl QueryRoot {
    async fn roots(&self, ctx: &Context<'_>) -> Vec<Root> {
        let roots = ctx.data_unchecked::<AppState>().roots.lock().await;
        roots.iter().map(|(_, root)| root).cloned().collect()
    }
}

#[Object]
impl MutationRoot {
    async fn create_root(&self, ctx: &Context<'_>, name: String) -> ID {
        let mut roots = ctx.data_unchecked::<AppState>().roots.lock().await;
        let entry = roots.vacant_entry();
        let id: ID = entry.key().into();
        let root = Root { id: id.clone(), name };
        entry.insert(root);
        SimpleBroker::publish(RootChanged { mutation_type: MutationType::Created, id: id.clone() });
        id
    }

    async fn delete_root(&self, ctx: &Context<'_>, id: ID) -> Result<bool> {
        let mut roots = ctx.data_unchecked::<AppState>().roots.lock().await;
        let id = id.parse::<usize>()?;
        if roots.contains(id) {
            roots.remove(id);
            SimpleBroker::publish(RootChanged { mutation_type: MutationType::Deleted, id: id.into() });
            Ok(true)
        } else {
            Ok(false)
        }
    }
}

#[derive(Clone)]
struct RootChanged {
    mutation_type: MutationType,
    id: ID,
}

#[Object]
impl RootChanged {
    async fn mutation_type(&self) -> MutationType {
        self.mutation_type
    }

    async fn id(&self) -> &ID {
        &self.id
    }

    async fn root(&self, ctx: &Context<'_>) -> Result<Option<Root>> {
        let roots = ctx.data_unchecked::<AppState>().roots.lock().await;
        let id = self.id.parse::<usize>()?;
        Ok(roots.get(id).cloned())
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

    async fn roots(&self, mutation_type: Option<MutationType>) -> impl Stream<Item = RootChanged> {
        SimpleBroker::<RootChanged>::subscribe().filter(move |event| {
            let res = if let Some(mutation_type) = mutation_type { event.mutation_type == mutation_type } else { true };
            async move { res }
        })
    }
}
