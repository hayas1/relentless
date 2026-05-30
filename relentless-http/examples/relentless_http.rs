use std::time::Duration;

use relentless::{
    report::Reporter,
    shot::job::{Job, JobSpec},
};
use relentless_http::{contract::HttpContract, service::ReqwestClient};
use relentless_http_dev_server::runner::RunCommand;
use reqwest::Body;

#[tokio::main]
async fn main() {
    let server = tokio::spawn(server());
    client().await;
    server.abort();
    let _ = server.await;
}

async fn server() {
    let rc = RunCommand { listen: "0.0.0.0".into(), port: "3333".into() };
    rc.serve().await.unwrap()
}

async fn client() {
    while reqwest::get("http://localhost:3333").await.is_err() {
        tokio::time::sleep(Duration::from_millis(200)).await;
    }
    let spec = JobSpec {
        destination: vec![
            ("actual".to_string(), "http://localhost:3333".parse().unwrap()),
            ("expect".to_string(), "http://localhost:3333".parse().unwrap()),
        ],
        ..Default::default()
    };
    let files: Result<Vec<_>, _> = glob::glob("relentless-http/examples/config/compare.yaml").unwrap().collect();
    let job = Job::from_files(&files.unwrap()).unwrap();
    let client = ReqwestClient::new().await.unwrap();
    let report = job.shot::<_, _, HttpContract<Body, Body>>(tower::make::Shared::new(client), &spec).await.unwrap();
    spec.report(&report).unwrap();
}
