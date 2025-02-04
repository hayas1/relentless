use async_graphql::{http::GraphiQLSource, Schema};
use async_graphql_axum::{GraphQL, GraphQLSubscription};
use axum::{
    response::{self, IntoResponse},
    routing::get,
    Router,
};

use crate::{
    book::{MutationRoot, QueryRoot, Storage, SubscriptionRoot},
    env::Env,
};

async fn graphiql() -> impl IntoResponse {
    response::Html(GraphiQLSource::build().endpoint("/").subscription_endpoint("/ws").finish())
}

pub fn app(env: Env) -> Router<()> {
    let _ = env;
    app_with()
}
// pub fn app_with(state: AppState) -> Router<()> {
pub fn app_with() -> Router<()> {
    router()
}
// pub fn router(state: AppState) -> Router<()> {
pub fn router() -> Router<()> {
    let schema = Schema::build(QueryRoot, MutationRoot, SubscriptionRoot).data(Storage::default()).finish();

    Router::new()
        .route("/", get(graphiql).post_service(GraphQL::new(schema.clone())))
        .route_service("/ws", GraphQLSubscription::new(schema))
}
