use std::process::ExitCode;

#[cfg(feature = "cli")]
#[tokio::main]
pub async fn main() -> Result<ExitCode, Box<dyn std::error::Error + Send + Sync>> {
    use relentless::interface::command::{Assault, Relentless};
    use relentless_grpc::{client::GrpcClient, command::GrpcAssault};

    let assault = GrpcAssault::new(Relentless::parse_cli());
    let (configs, errors) = assault.configs();
    errors.into_iter().for_each(|err| eprintln!("{err}"));
    let client = GrpcClient::new(&assault.all_destinations(&configs)).await?;
    let record = assault.build_service(client);
    Ok(assault.execute(record).await?)
}

#[cfg(not(feature = "cli"))]
pub fn main() -> Result<ExitCode, Box<dyn std::error::Error + Send + Sync>> {
    Err("cli feature is not enabled".into())
}
