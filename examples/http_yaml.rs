use relentless::config::Config;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let (worker, testcases) = Config::read("./examples/assault.yaml")?.instance()?;
    let result = worker.assault(testcases).await?;
    Ok(result)
}
