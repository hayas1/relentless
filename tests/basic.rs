use axum::body::Body;
use relentless::{
    command::{Relentless, ReportFormat},
    evaluate::DefaultEvaluator,
    report::Reportable,
};

use relentless_dev_server::route;

#[tokio::test]
#[cfg(all(feature = "json", feature = "yaml"))]
async fn test_example_yaml_config() {
    let relentless = Relentless {
        file: glob::glob("examples/config/*.yaml").unwrap().collect::<Result<Vec<_>, _>>().unwrap(),
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
    let report = relentless
        .assault_with::<_, Body, Body, _>(configs, &mut vec![services; relentless.file.len()], &DefaultEvaluator)
        .await
        .unwrap();

    assert_eq!(relentless.file.len(), report.sub_reportable().len());
    assert!(report.allow(false));
}

#[tokio::test]
#[cfg(all(feature = "json", feature = "yaml"))]
async fn test_basic_yaml_config() {
    let relentless = Relentless {
        file: glob::glob("tests/config/basic/*.yaml").unwrap().collect::<Result<Vec<_>, _>>().unwrap(),
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
    let report = relentless
        .assault_with::<_, Body, Body, _>(configs, &mut vec![services; relentless.file.len()], &DefaultEvaluator)
        .await
        .unwrap();

    assert_eq!(relentless.file.len(), report.sub_reportable().len());
    assert!(report.allow(false));
}

#[tokio::test]
#[cfg(all(feature = "json", feature = "toml"))]
async fn test_basic_toml_config() {
    let relentless = Relentless {
        file: glob::glob("tests/config/basic/*.toml").unwrap().collect::<Result<Vec<_>, _>>().unwrap(),
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
    let report = relentless
        .assault_with::<_, Body, Body, _>(configs, &mut vec![services; relentless.file.len()], &DefaultEvaluator)
        .await
        .unwrap();

    assert_eq!(relentless.file.len(), report.sub_reportable().len());
    assert!(report.allow(false));
}
