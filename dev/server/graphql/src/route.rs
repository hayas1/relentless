use async_graphql::{http::GraphiQLSource, Schema};
use async_graphql_axum::{GraphQL, GraphQLSubscription};
use axum::{
    response::{self, IntoResponse},
    routing::get,
    Router,
};

use crate::{
    book::{MutationRoot, QueryRoot, SubscriptionRoot},
    env::Env,
    state::AppState,
};

async fn graphiql() -> impl IntoResponse {
    response::Html(GraphiQLSource::build().endpoint("/").subscription_endpoint("/ws").finish())
}

pub fn app(env: Env) -> Router<()> {
    let state = AppState { env, ..Default::default() };
    app_with(state)
}
pub fn app_with(state: AppState) -> Router<()> {
    router(state)
}
pub fn router(state: AppState) -> Router<()> {
    let schema = Schema::build(QueryRoot, MutationRoot, SubscriptionRoot).data(state).finish();

    Router::new()
        .route("/", get(graphiql).post_service(GraphQL::new(schema.clone())))
        .route_service("/ws", GraphQLSubscription::new(schema))
}
