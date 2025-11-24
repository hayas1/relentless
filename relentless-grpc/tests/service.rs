use relentless::{
    report::ReportFormat,
    shot::{
        contract::Contract,
        job::{Job, JobSpec},
    },
};
use relentless_grpc::{request::GrpcRequest, response::GrpcResponse, service::DynamicContract, wip::JsonSerializer};
use relentless_grpc_dev_server::service::greeter::{pb::greeter_server::GreeterServer, GreeterImpl};
use tower::make::Shared;

#[tokio::test]
async fn test_example_yaml_config() {
    let spec = JobSpec { report_format: ReportFormat::NullDevice, ..Default::default() };
    let files: Result<Vec<_>, _> = glob::glob("examples/config/*.yaml").unwrap().collect();
    let job = Job::<GrpcRequest, GrpcResponse>::from_files(&files.unwrap()).unwrap();

    let server = Shared::new(GreeterServer::new(GreeterImpl));
    let report = job.shot(server, DynamicContract::<serde_json::Value, JsonSerializer>::new, &spec).await;

    assert!(report.pass());
}
