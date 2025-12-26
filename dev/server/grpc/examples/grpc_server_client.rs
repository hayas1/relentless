use std::time::Duration;

use relentless_grpc_dev_server::{
    app::{
        counter::pb::counter_client::CounterClient,
        greeter::pb::{greeter_client::GreeterClient, HelloRequest},
    },
    runner::RunCommand,
};
use tonic::transport::Channel;
use tonic_health::pb::{health_client::HealthClient, HealthCheckRequest};

#[tokio::main]
async fn main() {
    tokio::spawn(server());
    tokio::spawn(client()).await.unwrap();
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

    let mut health_client = HealthClient::new(channel);
    let health = health_client
        .check(HealthCheckRequest { service: "grpc.health.v1.Health".to_string() })
        .await
        .unwrap()
        .into_inner();
    println!("health: {health:?}");

    let mut hello_client = GreeterClient::connect("http://localhost:55555").await.unwrap();
    let hello = hello_client.say_hello(HelloRequest { name: "Rust".to_string() }).await.unwrap().into_inner();
    println!("hello: {hello:?}");

    let mut counter_client = CounterClient::connect("http://localhost:55555").await.unwrap();
    let counter = counter_client.increment(1).await.unwrap().into_inner();
    println!("counter: {counter:?}");
}
