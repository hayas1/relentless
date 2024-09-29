use relentless::command::Relentless;

#[tokio::main]
async fn main() -> Result<std::process::ExitCode, Box<dyn std::error::Error + Send + Sync>> {
    let cmd = Relentless { file: vec!["examples/config/compare.yaml".into()], ..Default::default() };
    let ret = cmd.assault().await?;
    Ok(ret.exit_code(cmd))
}
