use std::time::Duration;

use relentless::{
    report::Reporter,
    shot::job::{Job, JobSpec},
};
use relentless_grpc::{contract::DynamicContract, wip::JsonSerializer};
use relentless_grpc_dev_server::runner::RunCommand;
use tonic::transport::Channel;

#[tokio::main]
async fn main() {
    let server = tokio::spawn(server());
    client().await;
    server.abort();
    let _ = server.await;
}

async fn server() {
    let rc = RunCommand { listen: "0.0.0.0".into(), port: "55555".into() };
    rc.serve().await.unwrap()
}

async fn client() {
    let channel = loop {
        if let Ok(polling) = Channel::from_static("http://localhost:55555").connect().await {
            break polling;
        } else {
            tokio::time::sleep(Duration::from_millis(200)).await;
        }
    };

    let spec = JobSpec {
        destination: vec![
            ("actual".to_string(), "http://localhost:55555".parse().unwrap()),
            ("expect".to_string(), "http://localhost:55555".parse().unwrap()),
        ],
        ..Default::default()
    };
    let files: Result<Vec<_>, _> = glob::glob("relentless-grpc/examples/config/compare.yaml").unwrap().collect();
    let job = Job::from_files(&files.unwrap()).unwrap();
    let report = job
        .shot::<_, _, DynamicContract<serde_json::Value, JsonSerializer>>(tower::make::Shared::new(channel), &spec)
        .await
        .unwrap();
    spec.report(&report).unwrap();
}
