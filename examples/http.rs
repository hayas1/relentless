use relentless::{context::Context, Relentless};

#[tokio::main]
async fn main() -> Result<std::process::ExitCode, Box<dyn std::error::Error + Send + Sync>> {
    // let cmd = Default::default(); // TODO
    // let relentless = Relentless::read_dir(&cmd, "examples/config/").await?;
    // let outcome = relentless.assault(&cmd).await?;
    // outcome.report(&cmd)?;

    let outcome = Context::assault_with_config_dir("examples/config/")?.relentless_with_default_http_client().await?;
    Ok(outcome.exit_code(false))
}
