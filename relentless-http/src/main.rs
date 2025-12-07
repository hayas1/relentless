use std::process::ExitCode;

#[cfg(feature = "cli")]
#[tokio::main]
pub async fn main() -> Result<ExitCode, Box<dyn std::error::Error>> {
    use relentless::{
        record::metric::MeasureLayer,
        shot::{contract::Contract, job::Cli},
    };
    use relentless_http::service::{HttpContract, ReqwestClient};
    use tower::ServiceBuilder;

    let (job, spec) = Cli::job().await?;
    let measure = MeasureLayer::new();
    let client = ReqwestClient::<reqwest::Body, reqwest::Body>::new().await?;
    let service = ServiceBuilder::new().layer(&measure).service(client);
    let report = job.shot(tower::make::Shared::new(service), HttpContract::new, &spec).await?;
    dbg!(measure.aggregated().times());
    Ok((!report.pass() as u8).into())
}

#[cfg(not(feature = "cli"))]
pub fn main() -> Result<ExitCode, Box<dyn std::error::Error + Send + Sync>> {
    Err("cli feature is not enabled".into())
}
