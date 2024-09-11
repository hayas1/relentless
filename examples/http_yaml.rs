use relentless::testcase::config::Config;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let testcase = Config::import("./examples/assault.yaml")?;
    let result = testcase.run().await?;
    Ok(result)
}
