use async_graphql::{http::GraphiQLSource, Schema};
use async_graphql_axum::{GraphQL, GraphQLSubscription};
use axum::{
    response::{self, IntoResponse},
    routing::get,
    Router,
};

use crate::book::{MutationRoot, QueryRoot, Storage, SubscriptionRoot};

async fn graphiql() -> impl IntoResponse {
    response::Html(GraphiQLSource::build().endpoint("/").subscription_endpoint("/ws").finish())
}

// pub fn app(env: Env) -> NormalizePath<Router<()>> {
pub fn app() -> Router<()> {
    let schema = Schema::build(QueryRoot, MutationRoot, SubscriptionRoot).data(Storage::default()).finish();

    Router::new()
        .route("/", get(graphiql).post_service(GraphQL::new(schema.clone())))
        .route_service("/ws", GraphQLSubscription::new(schema))
}
