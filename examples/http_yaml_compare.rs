use relentless::Relentless;

#[tokio::main]
async fn main() -> Result<std::process::ExitCode, Box<dyn std::error::Error + Send + Sync>> {
    let cmd = Default::default(); // TODO
    let relentless = Relentless::read_paths(&cmd, vec!["examples/config/compare.yaml"]).await?;
    let result = relentless.assault(&cmd).await?;
    println!("{:#?}", result);
    Ok(result.exit_code(false))
}
