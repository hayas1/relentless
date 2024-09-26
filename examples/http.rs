use relentless::command::{Assault, Relentless};

#[tokio::main]
async fn main() -> Result<std::process::ExitCode, Box<dyn std::error::Error + Send + Sync>> {
    let relentless = Relentless {
        cmd: Assault { configs_dir: Some("examples/config".into()), ..Default::default() }.into(),
        ..Default::default()
    };
    let ret = relentless.execute().await?;
    Ok(ret.exit_code())
}
