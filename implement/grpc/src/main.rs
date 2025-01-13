use std::process::ExitCode;

#[cfg(feature = "cli")]
#[tokio::main]
pub async fn main() -> Result<ExitCode, Box<dyn std::error::Error + Send + Sync>> {
    use relentless::interface::command::{Assault, Relentless};
    use relentless_grpc::{client::DefaultGrpcClient, command::GrpcAssault};

    let assault = GrpcAssault { relentless: Relentless::parse_cli() };
    let client = DefaultGrpcClient::<serde_json::Value>::new();
    Ok(assault.execute(client).await?)
}

#[cfg(not(feature = "cli"))]
pub fn main() -> Result<ExitCode, Box<dyn std::error::Error + Send + Sync>> {
    Err("cli feature is not enabled".into())
}
