use std::process::ExitCode;

#[cfg(feature = "cli")]
#[tokio::main]
pub async fn main() -> Result<ExitCode, Box<dyn std::error::Error>> {
    use relentless::shot::job::Cli;
    use relentless_grpc::{client::GrpcChannel, request::GrpcRequest, response::GrpcResponse, wip::JsonSerializer};
    let report = Cli::shot::<_, _, GrpcRequest<serde_json::Value, JsonSerializer>, GrpcResponse>(GrpcChannel).await?;
    Ok((!report.pass() as u8).into())
}

#[cfg(not(feature = "cli"))]
pub fn main() -> Result<ExitCode, Box<dyn std::error::Error + Send + Sync>> {
    Err("cli feature is not enabled".into())
}
