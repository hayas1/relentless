use relentless::command::Assault;

#[tokio::main]
async fn main() -> Result<std::process::ExitCode, Box<dyn std::error::Error + Send + Sync>> {
    let relentless = Assault { configs_dir: Some("examples/config".into()), ..Default::default() };
    // let relentless = Relentless {
    //     cmd: Assault { configs_dir: Some("examples/config".into()), no_report: true, ..Default::default() }.into(),
    //     ..Default::default()
    // };
    let outcome = relentless.execute().await?;
    Ok(outcome.exit_code(relentless.strict))
}
