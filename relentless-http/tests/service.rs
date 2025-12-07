use axum::body::Body;
use relentless::{
    report::ReportFormat,
    shot::job::{Job, JobSpec},
};
use relentless_http::contract::HttpContract;

#[tokio::test]
async fn test_example_yaml_config() {
    let spec = JobSpec { report_format: ReportFormat::NullDevice, ..Default::default() };
    let files: Result<Vec<_>, _> = glob::glob("examples/config/*.yaml").unwrap().collect();
    let job = Job::<HttpContract<Body, Body>, _, _>::from_files(&files.unwrap()).unwrap();

    let app = relentless_http_dev_server::route::app_with(Default::default());
    let make = axum::ServiceExt::<axum::extract::Request>::into_make_service(app);
    let report = job.shot(make, &spec).await.unwrap();

    assert!(report.pass());
}
