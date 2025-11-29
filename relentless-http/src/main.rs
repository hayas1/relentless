use std::process::ExitCode;

#[cfg(feature = "cli")]
#[tokio::main]
pub async fn main() -> Result<ExitCode, Box<dyn std::error::Error>> {
    use relentless::shot::{contract::Contract, job::Cli};
    use relentless_http::service::{HttpContract, ReqwestClient};
    let (job, spec) = Cli::job().await?;
    let client = ReqwestClient::<reqwest::Body, reqwest::Body>::new().await?;
    let report = job.shot(tower::make::Shared::new(client), HttpContract::new, &spec).await?;
    Ok((!report.pass() as u8).into())
}

#[cfg(not(feature = "cli"))]
pub fn main() -> Result<ExitCode, Box<dyn std::error::Error + Send + Sync>> {
    Err("cli feature is not enabled".into())
}
