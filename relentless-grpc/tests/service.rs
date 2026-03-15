use relentless::{
    report::ReportFormat,
    shot::job::{Job, JobSpec},
};
use relentless_grpc::{contract::DynamicContract, wip::JsonSerializer};
use relentless_grpc_dev_server::runner::RunCommand;
use tower::make::Shared;

#[tokio::test]
async fn test_example_yaml_config() {
    let spec = JobSpec {
        report_format: ReportFormat::NullDevice,
        base_path: Some("..".parse().unwrap()),
        ..Default::default()
    };
    let files: Result<Vec<_>, _> = glob::glob("examples/config/*.yaml").unwrap().collect();
    let job = Job::from_files(&files.unwrap()).unwrap();

    let server = Shared::new(RunCommand::default().app().routes());
    let report = job.shot::<_, _, DynamicContract<serde_json::Value, JsonSerializer>>(server, &spec).await.unwrap();

    assert!(report.evaluated.allow);
}
