use std::process::ExitCode;

#[tokio::main]
#[cfg(all(feature = "yaml", feature = "json", feature = "console-report"))]
async fn main() -> Result<ExitCode, Box<dyn std::error::Error + Send + Sync>> {
    use axum::body::Body;
    use relentless::interface::command::{Assault, Relentless};
    use relentless_http::command::HttpAssault;
    use relentless_http_dev_server::route;

    let assault = HttpAssault::<Body, Body>::new(Relentless {
        file: vec![
            concat!(env!("CARGO_MANIFEST_DIR"), "/examples/config/assault.yaml").into(),
            concat!(env!("CARGO_MANIFEST_DIR"), "/examples/config/compare.yaml").into(),
        ],
        ..Default::default()
    });
    let (configs, errors) = assault.configs();
    errors.into_iter().for_each(|err| eprintln!("{}", err));

    let service = route::app_with(Default::default());
    let report = assault.assault_with(configs, service).await?;

    assault.report_with(&report, std::io::stdout())?;
    Ok(assault.exit_code(&report))
}

#[cfg(not(all(feature = "yaml", feature = "json", feature = "console-report")))]
fn main() -> ExitCode {
    eprintln!("Insufficient features for this example");
    ExitCode::FAILURE
}
