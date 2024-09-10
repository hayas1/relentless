use relentless::testcase::format::Testcases;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let testcase = Testcases::import("./examples/assault.yaml")?;
    let result = testcase.run().await?;
    Ok(result)
}
