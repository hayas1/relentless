use std::process::ExitCode;

#[cfg(feature = "cli")]
#[tokio::main]
pub async fn main() -> Result<ExitCode, Box<dyn std::error::Error>> {
    use relentless::shot::job::Cli;
    use relentless_http::service::{HttpContract, ReqwestClient};
    let client = ReqwestClient::<reqwest::Body, reqwest::Body>::new().await?;
    let make = tower::make::Shared::new(client);
    let report = Cli::shot::<_, _, HttpContract<_, _>>(make).await?;
    Ok((!report.pass() as u8).into())
}

#[cfg(not(feature = "cli"))]
pub fn main() -> Result<ExitCode, Box<dyn std::error::Error + Send + Sync>> {
    Err("cli feature is not enabled".into())
}
