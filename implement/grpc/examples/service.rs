use std::process::ExitCode;

#[tokio::main]
#[cfg(all(feature = "yaml", feature = "console-report"))]
async fn main() -> Result<ExitCode, Box<dyn std::error::Error + Send + Sync>> {
    use relentless::interface::command::{Assault, Relentless};
    use relentless_grpc::{client::DefaultGrpcClient, command::GrpcAssault};

    let assault = GrpcAssault::new(Relentless {
        file: vec!["examples/config/assault.yaml".into(), "examples/config/compare.yaml".into()],
        ..Default::default()
    });
    let (configs, errors) = assault.configs();
    errors.into_iter().for_each(|err| eprintln!("{}", err));

    let service = DefaultGrpcClient::new();
    let report = assault.assault_with(configs, service).await?;

    assault.report_with(&report, std::io::stdout())?;
    Ok(assault.exit_code(&report))
}

#[cfg(not(all(feature = "yaml", feature = "console-report")))]
fn main() -> ExitCode {
    eprintln!("Insufficient features for this example");
    ExitCode::FAILURE
}
