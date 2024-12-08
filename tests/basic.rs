use axum::body::Body;
use http::uri::Authority;
use relentless::{
    command::{Relentless, ReportFormat},
    evaluate::DefaultEvaluator,
    report::Reportable,
    service::origin_router::OriginRouter,
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
    let mut service = route::app_with(Default::default());
    let report =
        relentless.assault_with::<_, http::Request<Body>, _>(configs, &mut service, &DefaultEvaluator).await.unwrap();

    assert_eq!(relentless.file.len(), report.sub_reportable().len());
    assert!(relentless.allow(&report));
}

#[tokio::test]
#[cfg(all(feature = "json", feature = "yaml"))]
async fn test_basic_yaml_config() {
    let relentless = Relentless {
        file: glob::glob("tests/config/basic/*.yaml").unwrap().collect::<Result<Vec<_>, _>>().unwrap(),
        destination: vec![("actual".to_string(), "http://localhost:3001".to_string())],
        report_format: ReportFormat::NullDevice,
        ..Default::default()
    };
    let configs = relentless.configs().unwrap();
    let (actual, expect) = (route::app_with(Default::default()), route::app_with(Default::default()));
    let mut service = OriginRouter::new(
        [(Authority::from_static("localhost:3001"), actual), (Authority::from_static("localhost:3000"), expect)]
            .into_iter()
            .collect(),
    );
    let report =
        relentless.assault_with::<_, http::Request<Body>, _>(configs, &mut service, &DefaultEvaluator).await.unwrap();

    assert_eq!(relentless.file.len(), report.sub_reportable().len());
    assert!(relentless.allow(&report));
}

#[tokio::test]
#[cfg(all(feature = "json", feature = "toml"))]
async fn test_basic_toml_config() {
    let relentless = Relentless {
        file: glob::glob("tests/config/basic/*.toml").unwrap().collect::<Result<Vec<_>, _>>().unwrap(),
        destination: vec![("actual".to_string(), "http://localhost:3001".to_string())],
        report_format: ReportFormat::NullDevice,
        ..Default::default()
    };
    let configs = relentless.configs().unwrap();
    let (actual, expect) = (route::app_with(Default::default()), route::app_with(Default::default()));
    let mut service = OriginRouter::new(
        [(Authority::from_static("localhost:3001"), actual), (Authority::from_static("localhost:3000"), expect)]
            .into_iter()
            .collect(),
    );
    let report =
        relentless.assault_with::<_, http::Request<Body>, _>(configs, &mut service, &DefaultEvaluator).await.unwrap();

    relentless.report(&report).unwrap();
    assert_eq!(relentless.file.len(), report.sub_reportable().len());
    assert!(relentless.allow(&report));
}
