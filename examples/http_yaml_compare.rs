use relentless::command::Assault;

#[tokio::main]
async fn main() -> Result<std::process::ExitCode, Box<dyn std::error::Error + Send + Sync>> {
    let relentless = Assault { file: vec!["examples/config/compare.yaml".into()], ..Default::default() };
    let outcome = relentless.execute().await?;
    Ok(outcome.exit_code(relentless.strict))
}
