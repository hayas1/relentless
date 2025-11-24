use std::process::ExitCode;

#[cfg(feature = "cli")]
#[tokio::main]
pub async fn main() -> Result<ExitCode, Box<dyn std::error::Error>> {
    use relentless::shot::{contract::Contract, job::Cli};
    use relentless_grpc::{
        service::{DynamicContract, MakeChannel},
        wip::JsonSerializer,
    };
    let report = Cli::shot(MakeChannel, DynamicContract::<serde_json::Value, JsonSerializer>::new).await;
    Ok((!report.pass() as u8).into())
}

#[cfg(not(feature = "cli"))]
pub fn main() -> Result<ExitCode, Box<dyn std::error::Error + Send + Sync>> {
    Err("cli feature is not enabled".into())
}
