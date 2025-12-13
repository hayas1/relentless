use std::process::ExitCode;

#[cfg(feature = "cli")]
#[tokio::main]
pub async fn main() -> Result<ExitCode, Box<dyn std::error::Error>> {
    use relentless::{record::metric::MeasureLayer, report::Reporter, shot::job::Cli};
    use relentless_http::{contract::HttpContract, service::ReqwestClient};
    use reqwest::Body;
    use tower::ServiceBuilder;

    let (job, spec) = Cli::job().await?;
    let measure = MeasureLayer::new();
    let client = ReqwestClient::new().await?;
    let service = ServiceBuilder::new().layer(&measure).service(client);
    let report = job.shot::<_, _, HttpContract<Body, Body>>(tower::make::Shared::new(service), &spec).await?;
    spec.report_format.report(&report)?;
    dbg!(measure.aggregated().times());
    Ok((!report.evaluated.pass as u8).into())
}

#[cfg(not(feature = "cli"))]
pub fn main() -> Result<ExitCode, Box<dyn std::error::Error + Send + Sync>> {
    Err("cli feature is not enabled".into())
}
