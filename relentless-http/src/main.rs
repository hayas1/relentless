use std::process::ExitCode;

#[cfg(feature = "cli")]
#[tokio::main]
pub async fn main() -> Result<ExitCode, Box<dyn std::error::Error>> {
    use relentless::shot::job::Cli;
    use relentless_http::{client::HttpClient, request::HttpRequest, response::HttpResponse};
    let report = Cli::shot::<HttpClient<reqwest::Body, reqwest::Body>, HttpRequest, HttpResponse>().await?;
    Ok((!report.pass() as u8).into())
}

#[cfg(not(feature = "cli"))]
pub fn main() -> Result<ExitCode, Box<dyn std::error::Error + Send + Sync>> {
    Err("cli feature is not enabled".into())
}
