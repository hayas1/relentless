use relentless::Relentless;

#[tokio::main]
async fn main() -> Result<std::process::ExitCode, Box<dyn std::error::Error + Send + Sync>> {
    let relentless = Relentless::read_dir("examples/config/")?;
    let result = relentless.assault().await?;
    println!("{:#?}", result);
    Ok(result.exit_code(false))
}
