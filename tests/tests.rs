use axum::body::Body;
use relentless::command::{Assault, Relentless};

use example_http_server::{route, state::AppState};

#[tokio::test]
async fn test_example_assault() {
    let relentless = Relentless {
        cmd: Assault { file: vec!["examples/config/assault.yaml".into()], no_report: true, ..Default::default() }
            .into(),
        ..Default::default()
    };
    let services = [("test-api".to_string(), route::app(AppState { env: Default::default() }))].into_iter().collect();
    let outcome = relentless.assault_with::<_, Body, Body>(vec![services]).await.unwrap();

    assert!(outcome.allow(false));
}
