#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let url = "http://localhost:3000";
    let contents = reqwest::get(url).await?.text().await?;
    println!("{:?}", contents);
    Ok(())
}
