use axum::body::Body;
use http::uri::Authority;
use relentless::{
    assault::{reportable::Reportable, service::origin_router::OriginRouter},
    interface::command::{Relentless, ReportFormat},
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
    let service = route::app_with(Default::default());
    let report = relentless.assault_with::<_, http::Request<Body>>(configs, service).await.unwrap();

    assert_eq!(relentless.file.len(), report.sub_reportable().len());
    assert!(relentless.allow(&report));
}

#[test]
#[cfg(all(feature = "json", feature = "toml"))]
fn test_same_basic_yaml_toml_config() {
    let yaml = glob::glob("tests/config/basic/*.yaml").unwrap().collect::<Result<Vec<_>, _>>().unwrap();
    let toml = glob::glob("tests/config/basic/*.toml").unwrap().collect::<Result<Vec<_>, _>>().unwrap();
    assert_eq!(yaml.len(), toml.len());

    let yam = Relentless { file: yaml, ..Default::default() };
    let tom = Relentless { file: toml, ..Default::default() };
    assert_json_diff::assert_json_eq!(yam.configs().unwrap(), tom.configs().unwrap());
    assert_eq!(yam.configs().unwrap(), tom.configs().unwrap());
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
    let service = OriginRouter::new(
        [(Authority::from_static("localhost:3001"), actual), (Authority::from_static("localhost:3000"), expect)]
            .into_iter()
            .collect(),
    );
    let report = relentless.assault_with::<_, http::Request<Body>>(configs, service).await.unwrap();

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
    let service = OriginRouter::new(
        [(Authority::from_static("localhost:3001"), actual), (Authority::from_static("localhost:3000"), expect)]
            .into_iter()
            .collect(),
    );
    let report = relentless.assault_with::<_, http::Request<Body>>(configs, service).await.unwrap();

    relentless.report(&report).unwrap();
    assert_eq!(relentless.file.len(), report.sub_reportable().len());
    assert!(relentless.allow(&report));
}
