use std::process::ExitCode;

#[tokio::main]
#[cfg(all(feature = "yaml", feature = "json", feature = "console-report"))]
async fn main() -> Result<ExitCode, Box<dyn std::error::Error + Send + Sync>> {
    use axum::{body::Body, extract::Request};
    use relentless::{interface::command::Relentless, Error};
    use relentless_dev_server::route;

    let cmd = Relentless {
        file: vec!["examples/config/assault.yaml".into(), "examples/config/compare.yaml".into()],
        ..Default::default()
    };
    let configs = cmd.configs().map_err(Error::from)?;
    let service = route::app_with(Default::default());
    let report = cmd.assault_with::<_, Request<Body>>(configs, service).await?;

    cmd.report(&report)?;
    Ok(report.exit_code(&cmd))
}

#[cfg(not(all(feature = "yaml", feature = "json", feature = "console-report")))]
fn main() -> ExitCode {
    eprintln!("Insufficient features for this example");
    ExitCode::FAILURE
}
