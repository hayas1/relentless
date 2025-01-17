use std::process::ExitCode;

#[cfg(feature = "cli")]
#[tokio::main]
pub async fn main() -> Result<ExitCode, Box<dyn std::error::Error + Send + Sync>> {
    use relentless::interface::command::{Assault, Relentless};
    use relentless_http::{client::DefaultHttpClient, command::HttpAssault};

    let assault = HttpAssault::new(Relentless::parse_cli());
    let client = DefaultHttpClient::<reqwest::Body, reqwest::Body>::new().await?;
    let record = assault.build_service(client);
    Ok(assault.execute(record).await?)
}

#[cfg(not(feature = "cli"))]
pub fn main() -> Result<ExitCode, Box<dyn std::error::Error + Send + Sync>> {
    Err("cli feature is not enabled".into())
}
