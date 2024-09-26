use axum::body::Body;
use relentless::{
    command::{Assault, Cmd},
    config::Config,
    context::ContextBuilder,
    worker::{Control, Worker},
};

use example_http_server::{route, state::AppState};

#[tokio::test]
async fn test_example_assault() {
    // let (config, service) =
    //     (Config::read("examples/config/assault.yaml").unwrap(), route::app(AppState { env: Default::default() }));
    // let services = vec![("test-api".to_string(), service)].into_iter().collect();
    // let relentless =
    //     Control::<_, Body, Body>::new(vec![config.clone()], vec![Worker::new(config.worker_config, services).unwrap()]);
    // let result = relentless.assault(&Default::default()).await.unwrap();

    // let result = ContextBuilder::assault_with_config(vec![config])
    //     .relentless_with_service::<_, Body, Body>(vec![services])
    //     .await
    //     .unwrap();

    let relentless = Assault { file: vec!["examples/config/assault.yaml".into()], ..Default::default() };
    let services = [("test-api".to_string(), route::app(AppState { env: Default::default() }))].into_iter().collect();
    let outcome = relentless.execute_with::<_, Body, Body>(vec![services]).await.unwrap();

    assert!(outcome.allow(false));
}
