pub mod content;

use async_graphql::{http::GraphiQLSource, Context, Enum, Object, Schema, Subscription, ID};
use async_graphql_axum::{GraphQL, GraphQLSubscription};
use axum::{response::Html, routing::get, Router};
use futures::{Stream, StreamExt};

use crate::{env::Env, simple_broker::SimpleBroker, state::AppState};

pub fn app(env: Env) -> Router<()> {
    let state = AppState { env, ..Default::default() };
    app_with(state)
}
pub fn app_with(state: AppState) -> Router<()> {
    router(state)
}
pub fn router(state: AppState) -> Router<()> {
    let schema = Schema::build(QueryRoot, MutationRoot, SubscriptionRoot).data(state).finish();

    let graphiql = || async move { Html(GraphiQLSource::build().endpoint("/").subscription_endpoint("/ws").finish()) };

    Router::new()
        .route("/", get(graphiql).post_service(GraphQL::new(schema.clone())))
        .route_service("/ws", GraphQLSubscription::new(schema))
}

pub struct QueryRoot;
#[Object]
impl QueryRoot {
    async fn content(&self, ctx: &Context<'_>, id: ID) -> Option<content::Content> {
        let contents = ctx.data_unchecked::<AppState>().contents.lock().await;
        let id = id.parse::<usize>().ok()?;
        contents.get(id).cloned()
    }
    async fn contents(&self, ctx: &Context<'_>) -> Vec<content::Content> {
        let contents = ctx.data_unchecked::<AppState>().contents.lock().await;
        contents.iter().map(|(_, content)| content).cloned().collect()
    }
}

pub struct MutationRoot;
#[Object]
impl MutationRoot {
    async fn create_content(&self, ctx: &Context<'_>, name: String) -> ID {
        let mut contents = ctx.data_unchecked::<AppState>().contents.lock().await;
        let entry = contents.vacant_entry();
        let id: ID = entry.key().into();
        let content = content::Content { id: id.clone(), name };
        entry.insert(content);
        SimpleBroker::publish(content::ContentChanged { mutation_type: MutationType::Created, id: id.clone() });
        id
    }

    async fn delete_content(&self, ctx: &Context<'_>, id: ID) -> async_graphql::Result<bool> {
        let mut contents = ctx.data_unchecked::<AppState>().contents.lock().await;
        let id = id.parse::<usize>()?;
        if contents.contains(id) {
            contents.remove(id);
            SimpleBroker::publish(content::ContentChanged { mutation_type: MutationType::Deleted, id: id.into() });
            Ok(true)
        } else {
            Ok(false)
        }
    }
}

#[derive(Enum, Eq, PartialEq, Copy, Clone)]
pub enum MutationType {
    Created,
    Deleted,
}

pub struct SubscriptionRoot;

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

    async fn contents(&self, mutation_type: Option<MutationType>) -> impl Stream<Item = content::ContentChanged> {
        SimpleBroker::<content::ContentChanged>::subscribe().filter(move |event| {
            let res = if let Some(mutation_type) = mutation_type { event.mutation_type == mutation_type } else { true };
            async move { res }
        })
    }
}
