use std::process::ExitCode;

#[cfg(feature = "cli")]
#[tokio::main]
pub async fn main() -> Result<ExitCode, Box<dyn std::error::Error + Send + Sync>> {
    use clap::Parser;
    use relentless::interface::command::{Assault, Relentless};
    use relentless_http::{client::DefaultHttpClient, command::HttpAssault};

    let assault = HttpAssault { relentless: Relentless::parse() };
    let client = DefaultHttpClient::<reqwest::Body, reqwest::Body>::new().await?;
    Ok(assault.execute(client).await?)
}

#[cfg(not(feature = "cli"))]
pub fn main() -> Result<ExitCode, Box<dyn std::error::Error + Send + Sync>> {
    Err("cli feature is not enabled".into())
}
