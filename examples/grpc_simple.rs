use std::process::ExitCode;

#[tokio::main]
#[cfg(all(feature = "yaml", feature = "json", feature = "console-report"))]
async fn main() -> Result<ExitCode, Box<dyn std::error::Error + Send + Sync>> {
    use relentless::interface::command::Relentless;

    let cmd = Relentless {
        file: vec!["examples/wip/wip_grpc.yaml".into()],
        output_record: Some("./record".into()),
        ..Default::default()
    };

    let report = cmd.assault_grpc().await?;

    cmd.report(&report)?;
    Ok(report.exit_code(&cmd))
}

#[cfg(not(all(feature = "yaml", feature = "json", feature = "console-report")))]
fn main() -> ExitCode {
    eprintln!("Insufficient features for this example");
    ExitCode::FAILURE
}
