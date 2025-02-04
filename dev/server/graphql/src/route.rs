pub mod book;

use async_graphql::{http::GraphiQLSource, Schema};
use async_graphql_axum::{GraphQL, GraphQLSubscription};
use axum::{response::Html, routing::get, Router};

use crate::{
    env::Env,
    route::book::{MutationRoot, QueryRoot, SubscriptionRoot},
    state::AppState,
};

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
