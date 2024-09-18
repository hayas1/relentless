use axum::{
    body::{to_bytes, Body, Bytes, HttpBody},
    http::{HeaderMap, Request, StatusCode},
};
use relentless::{
    config::{Config, Testcase},
    Relentless,
};
use serde::de::DeserializeOwned;
use tower::ServiceExt;

use example_http_server::{
    route::{self, health::Health},
    state::AppState,
};

#[tokio::test]
async fn test_example_assault() -> Result<(), Box<dyn std::error::Error>> {
    let (config, service) =
        (Config::read("examples/config/assault.yaml")?, route::app(AppState { env: Default::default() }));
    let relentless = Relentless::<_, http::Request<Body>, http::Response<Body>>::new(vec![config], Some(service));
    let result = relentless.assault().await?;

    Ok(())
}
