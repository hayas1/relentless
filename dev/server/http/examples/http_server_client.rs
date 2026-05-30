use std::time::Duration;

use relentless_http_dev_server::{
    app::{counter::CounterResponse, health::Health},
    runner::RunCommand,
};

#[tokio::main]
async fn main() {
    tokio::spawn(server());
    tokio::spawn(client()).await.unwrap();
}

async fn server() {
    let rc = RunCommand { listen: "0.0.0.0".into(), port: "3333".into() };
    rc.serve().await.unwrap()
}

async fn client() {
    while reqwest::get("http://localhost:3333").await.is_err() {
        tokio::time::sleep(Duration::from_millis(200)).await;
    }

    let hello: String = reqwest::get("http://localhost:3333").await.unwrap().text().await.unwrap();
    println!("hello: {hello}");

    let health: Health = reqwest::get("http://localhost:3333/health/rich").await.unwrap().json().await.unwrap();
    println!("health: {health:?}");

    let counter: CounterResponse<i64> =
        reqwest::get("http://localhost:3333/counter/increment").await.unwrap().json().await.unwrap();
    println!("counter: {counter:?}");
}
