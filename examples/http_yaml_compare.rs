use relentless::command::Relentless;

#[tokio::main]
#[cfg(feature = "default-http-client")]
async fn main() -> Result<std::process::ExitCode, Box<dyn std::error::Error + Send + Sync>> {
    let cmd = Relentless { file: vec!["examples/config/compare.yaml".into()], ..Default::default() };
    let ret = cmd.assault().await?;
    Ok(ret.exit_code(cmd))
}

#[tokio::main]
#[cfg(not(feature = "default-http-client"))]
async fn main() -> Result<std::process::ExitCode, Box<dyn std::error::Error + Send + Sync>> {
    todo!();
}
