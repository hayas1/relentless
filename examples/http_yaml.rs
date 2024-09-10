use relentless::testcase::format::Testcase;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let testcase = Testcase::import("./examples/assault.yaml")?;
    let result = testcase.run().await?;
    Ok(result)
}
