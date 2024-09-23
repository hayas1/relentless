use relentless::Relentless;

#[tokio::main]
async fn main() -> Result<std::process::ExitCode, Box<dyn std::error::Error + Send + Sync>> {
    let cmd = Default::default(); // TODO
    let relentless = Relentless::read_dir(&cmd, "examples/config/").await?;
    let outcome = relentless.assault(&cmd).await?;
    outcome.report(&cmd)?;
    Ok(outcome.exit_code(false))
}
