use std::process::ExitCode;

#[cfg(feature = "cli")]
#[tokio::main]
pub async fn main() -> Result<ExitCode, Box<dyn std::error::Error>> {
    use relentless::{
        report::Reporter,
        shot::job::{Cli, Job},
    };
    use relentless_grpc::{
        contract::DynamicContract, interceptor::OtelInterceptor, service::MakeChannel, wip::JsonSerializer,
    };

    Cli::run(|job: Job<_, _, _>, spec| async move {
        // let measure = MeasureLayer::new();
        let otel = OtelInterceptor;
        let make = MakeChannel(otel);
        let report = job.shot::<_, _, DynamicContract<serde_json::Value, JsonSerializer>>(make, &spec).await?;
        spec.report(&report)?;
        // dbg!(measure.aggregated().times());
        Ok((!report.evaluated.assess().success() as u8).into())
    })
    .await
}

#[cfg(not(feature = "cli"))]
pub fn main() -> Result<ExitCode, Box<dyn std::error::Error + Send + Sync>> {
    Err("cli feature is not enabled".into())
}
