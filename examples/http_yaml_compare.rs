use relentless::command::Relentless;

#[tokio::main]
async fn main() -> Result<std::process::ExitCode, Box<dyn std::error::Error + Send + Sync>> {
    let relentless = Relentless { file: vec!["examples/config/compare.yaml".into()], ..Default::default() };
    let ret = relentless.assault().await?;
    Ok(ret.exit_code(false))
}
