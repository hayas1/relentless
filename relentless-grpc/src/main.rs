use std::process::ExitCode;

#[cfg(feature = "cli")]
#[tokio::main]
pub async fn main() -> Result<ExitCode, Box<dyn std::error::Error>> {
    use relentless::shot::{contract::Contract, job::Cli};
    use relentless_grpc::{
        service::{DynamicContract, MakeChannel},
        wip::JsonSerializer,
    };
    let (job, spec) = Cli::job().await?;
    let report = job.shot(MakeChannel, DynamicContract::<serde_json::Value, JsonSerializer>::new, &spec).await?;
    Ok((!report.pass() as u8).into())
}

#[cfg(not(feature = "cli"))]
pub fn main() -> Result<ExitCode, Box<dyn std::error::Error + Send + Sync>> {
    Err("cli feature is not enabled".into())
}
