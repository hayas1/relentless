use axum::body::Body;
use relentless::command::Assault;

use example_http_server::{route, state::AppState};

#[tokio::test]
async fn test_example_assault() {
    let mut relentless = Assault { file: vec!["examples/config/assault.yaml".into()], ..Default::default() };
    relentless.no_report = true; // TODO builder pattern ?
    let services = [("test-api".to_string(), route::app(AppState { env: Default::default() }))].into_iter().collect();
    let outcome = relentless.execute_with::<_, Body, Body>(vec![services]).await.unwrap();

    assert!(outcome.allow(false));
}
