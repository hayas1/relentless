#![cfg(all(feature = "json", feature = "yaml"))]

use axum::body::Body;
use relentless::{
    command::{Relentless, ReportFormat},
    evaluate::DefaultEvaluator,
    outcome::Reportable,
};

use example_http_server::route;

#[tokio::test]
async fn test_example_assault() {
    let relentless = Relentless {
        file: vec!["examples/config/assault.yaml".into()],
        report_format: ReportFormat::NullDevice,
        ..Default::default()
    };
    let configs = relentless.configs().unwrap();
    let services = [("test-api".to_string(), route::app_with(Default::default()))].into_iter().collect();
    let outcome =
        relentless.assault_with::<_, Body, Body, _>(configs, vec![services], &DefaultEvaluator).await.unwrap();

    assert!(outcome.allow(false));
}
