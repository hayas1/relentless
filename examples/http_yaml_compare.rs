use relentless::Relentless;

#[tokio::main]
async fn main() -> Result<std::process::ExitCode, Box<dyn std::error::Error + Send + Sync>> {
    let mut relentless = Relentless::read_paths(vec!["examples/config/compare.yaml"]).await?;
    let result = relentless.assault().await?;
    println!("{:#?}", result);
    Ok(result.exit_code(false))
}
