use std::time::Duration;

use relentless_http_dev_server::{
    app::{counter::CounterResponse, health::Health},
    runner::RunCommand,
};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    tokio::spawn(server());
    wait().await;
    tokio::spawn(client()).await.unwrap();
    Ok(())
}

async fn server() {
    let rc = RunCommand { listen: "0.0.0.0".into(), port: "3030".into() };
    rc.serve().await.unwrap()
}

async fn wait() {
    while reqwest::get("http://localhost:3030").await.is_err() {
        tokio::time::sleep(Duration::from_millis(200)).await;
    }
}

async fn client() {
    let hello: String = reqwest::get("http://localhost:3030").await.unwrap().text().await.unwrap();
    println!("hello: {hello}");

    let health: Health = reqwest::get("http://localhost:3030/health/rich").await.unwrap().json().await.unwrap();
    println!("health: {health:?}");

    let counter: CounterResponse<i64> =
        reqwest::get("http://localhost:3030/counter/increment").await.unwrap().json().await.unwrap();
    println!("counter: {counter:?}");
}
