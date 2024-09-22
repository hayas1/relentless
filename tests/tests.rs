use axum::{
    body::{to_bytes, Body, Bytes, HttpBody},
    http::{HeaderMap, Request, StatusCode},
};
use http_body_util::Empty;
use hyper::body::Incoming;
use relentless::{
    config::{BodyStructure, Config, Testcase},
    worker::Worker,
    Relentless_,
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
    let services = vec![("test-api".to_string(), service)].into_iter().collect();
    let relentless =
        Relentless_::<_, Body, Body>::new(vec![config.clone()], vec![Worker::new(config.worker_config, services)?]);
    let result = relentless.assault().await?;

    assert!(result.pass());
    Ok(())
}
