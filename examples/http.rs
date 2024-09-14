use relentless::Relentless;

#[tokio::main]
async fn main() -> Result<std::process::ExitCode, Box<dyn std::error::Error + Send + Sync>> {
    let relentless = Relentless::read_dir("./examples/")?;
    let result = relentless.assault().await?;
    Ok(result.exit_code())
}
