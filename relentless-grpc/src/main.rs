use std::process::ExitCode;

#[cfg(feature = "cli")]
#[tokio::main]
pub async fn main() -> Result<ExitCode, Box<dyn std::error::Error>> {
    use relentless::shot::job::Cli;
    use relentless_grpc::service::{DescriptorLayer, MakeChannel};
    let report = Cli::shot::<_, _, DescriptorLayer<serde_json::Value, _>>(MakeChannel).await?;
    Ok((!report.pass() as u8).into())
}

#[cfg(not(feature = "cli"))]
pub fn main() -> Result<ExitCode, Box<dyn std::error::Error + Send + Sync>> {
    Err("cli feature is not enabled".into())
}
