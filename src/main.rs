use client::worker::run;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let client = reqwest::Client::new();
    let response = run(client).await?;
    println!("{:?}", response.text().await?);
    Ok(())
}
