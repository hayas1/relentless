use axum::body::Body;
use relentless::{command::Relentless, config::Destinations};

use example_http_server::route;

#[tokio::test]
async fn test_example_assault() {
    let relentless =
        Relentless { file: vec!["examples/config/assault.yaml".into()], no_report: true, ..Default::default() };
    let services = Destinations([("test-api".to_string(), route::app_with(Default::default()))].into_iter().collect());
    let outcome = relentless.assault_with::<_, Body, Body>(vec![services]).await.unwrap();

    assert!(outcome.allow(false));
}
