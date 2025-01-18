use axum::body::Body;
use http::uri::Authority;
use relentless::{
    assault::{reportable::Reportable, service::origin_router::OriginRouter},
    interface::command::{Assault, Relentless, ReportFormat},
};

use relentless_http::command::HttpAssault;
use relentless_http_dev_server::route;

#[tokio::test]
#[cfg(all(feature = "json", feature = "yaml"))]
async fn test_example_yaml_config() {
    let assault = HttpAssault::<Body, Body>::new(Relentless {
        file: glob::glob("examples/config/*.yaml").unwrap().collect::<Result<Vec<_>, _>>().unwrap(),
        report_format: ReportFormat::NullDevice,
        ..Default::default()
    });
    let (configs, _) = assault.configs();
    let service = route::app_with(Default::default());
    let report = assault.assault_with(configs, service).await.unwrap();

    assert_eq!(assault.command().file.len(), report.sub_reportable().unwrap().len());
    assert!(assault.allow(&report));
}

#[test]
#[cfg(all(feature = "json", feature = "yaml", feature = "toml"))]
fn test_same_basic_yaml_toml_config() {
    let yaml = glob::glob("tests/config/basic/*.yaml").unwrap().collect::<Result<Vec<_>, _>>().unwrap();
    let toml = glob::glob("tests/config/basic/*.toml").unwrap().collect::<Result<Vec<_>, _>>().unwrap();
    assert_eq!(yaml.len(), toml.len());

    let yam = HttpAssault::<Body, Body>::new(Relentless { file: yaml, ..Default::default() });
    let tom = HttpAssault::<Body, Body>::new(Relentless { file: toml, ..Default::default() });
    assert_json_diff::assert_json_eq!(yam.configs().0, tom.configs().0,);
    assert_eq!(yam.configs().0, tom.configs().0);
}

#[tokio::test]
#[cfg(all(feature = "json", feature = "yaml"))]
async fn test_basic_yaml_config() {
    let assault = HttpAssault::<Body, Body>::new(Relentless {
        file: glob::glob("tests/config/basic/*.yaml").unwrap().collect::<Result<Vec<_>, _>>().unwrap(),
        destination: vec![("actual".to_string(), "http://localhost:3001".to_string())],
        report_format: ReportFormat::NullDevice,
        ..Default::default()
    });
    let (configs, _) = assault.configs();
    let (actual, expect) = (route::app_with(Default::default()), route::app_with(Default::default()));
    let service = OriginRouter::new(
        [(Authority::from_static("localhost:3001"), actual), (Authority::from_static("localhost:3000"), expect)]
            .into_iter()
            .collect(),
    );
    let report = assault.assault_with(configs, service).await.unwrap();

    assert_eq!(assault.command().file.len(), report.sub_reportable().unwrap().len());
    assert!(assault.allow(&report));
}

#[tokio::test]
#[cfg(all(feature = "json", feature = "toml"))]
async fn test_basic_toml_config() {
    let assault = HttpAssault::<Body, Body>::new(Relentless {
        file: glob::glob("tests/config/basic/*.toml").unwrap().collect::<Result<Vec<_>, _>>().unwrap(),
        destination: vec![("actual".to_string(), "http://localhost:3001".to_string())],
        report_format: ReportFormat::NullDevice,
        ..Default::default()
    });
    let (configs, _) = assault.configs();
    let (actual, expect) = (route::app_with(Default::default()), route::app_with(Default::default()));
    let service = OriginRouter::new(
        [(Authority::from_static("localhost:3001"), actual), (Authority::from_static("localhost:3000"), expect)]
            .into_iter()
            .collect(),
    );
    let report = assault.assault_with(configs, service).await.unwrap();

    assert_eq!(assault.command().file.len(), report.sub_reportable().unwrap().len());
    assert!(assault.allow(&report));
}
