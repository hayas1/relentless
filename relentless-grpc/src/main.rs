use std::process::ExitCode;

#[cfg(feature = "cli")]
#[tokio::main]
pub async fn main() -> Result<ExitCode, Box<dyn std::error::Error>> {
    use relentless::{
        record::metric::MeasureLayer,
        report::Reporter,
        shot::job::{Cli, Job},
    };
    use relentless_grpc::{
        contract::{DynamicContract, GrpcDescriptor},
        request::GrpcRequest,
        response::GrpcResponse,
        service::MakeChannel,
        wip::JsonSerializer,
    };

    Cli::run(|job: Job<GrpcDescriptor, GrpcRequest, GrpcResponse>, spec| async move {
        let measure = MeasureLayer::new();
        let make = MakeChannel(measure.clone());
        let report = job.shot::<_, _, DynamicContract<serde_json::Value, JsonSerializer>>(make, &spec).await?;
        spec.report_format.report(&report)?;
        dbg!(measure.aggregated().times());
        Ok((!report.evaluated.pass as u8).into())
    })
    .await
}

#[cfg(not(feature = "cli"))]
pub fn main() -> Result<ExitCode, Box<dyn std::error::Error + Send + Sync>> {
    Err("cli feature is not enabled".into())
}
