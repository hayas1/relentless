#![cfg(all(feature = "json", feature = "yaml"))]

use axum::body::Body;
use relentless::{
    command::{Relentless, ReportFormat},
    evaluate::DefaultEvaluator,
    report::Reportable,
};

use relentless_dev_server::route;

#[tokio::test]
async fn test_example_assault() {
    let relentless = Relentless {
        file: glob::glob("tests/config/basic/*").unwrap().collect::<Result<Vec<_>, _>>().unwrap(),
        report_format: ReportFormat::NullDevice,
        ..Default::default()
    };
    let configs = relentless.configs().unwrap();
    let services = [
        ("test-api".to_string(), route::app_with(Default::default())),
        ("expect".to_string(), route::app_with(Default::default())),
        ("actual".to_string(), route::app_with(Default::default())),
    ]
    .into_iter()
    .collect();
    let report = relentless.assault_with::<_, Body, Body, _>(configs, vec![services], &DefaultEvaluator).await.unwrap();

    assert!(report.allow(false));
}
