use axum::body::Body;
use relentless::{
    config::Config,
    context::Context,
    worker::{Control, Worker},
};

use example_http_server::{route, state::AppState};

#[tokio::test]
async fn test_example_assault() {
    let (config, service) =
        (Config::read("examples/config/assault.yaml").unwrap(), route::app(AppState { env: Default::default() }));
    let services = vec![("test-api".to_string(), service)].into_iter().collect();
    // let relentless =
    //     Control::<_, Body, Body>::new(vec![config.clone()], vec![Worker::new(config.worker_config, services).unwrap()]);
    // let result = relentless.assault(&Default::default()).await.unwrap();

    let result = Context::assault_with_config(vec![config])
        .relentless_with_service::<_, Body, Body>(vec![services])
        .await
        .unwrap();

    assert!(result.allow(false));
}
