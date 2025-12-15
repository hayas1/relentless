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
    let job = Job::from_files(&files.unwrap()).unwrap();

    let service = relentless_http_dev_server::route::AppRouter::default().service();
    let make = axum::ServiceExt::<axum::extract::Request>::into_make_service(service);
    let report = job.shot::<_, _, HttpContract<Body, Body>>(make, &spec).await.unwrap();

    assert!(report.evaluated.pass);
}
