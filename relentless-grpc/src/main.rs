use std::process::ExitCode;

#[cfg(feature = "cli")]
#[tokio::main]
pub async fn main() -> Result<ExitCode, Box<dyn std::error::Error>> {
    use relentless::{record::metric::MeasureLayer, shot::job::Cli};
    use relentless_grpc::{
        contract::{DynamicContract, GrpcDescriptor},
        service::MakeChannel,
        wip::JsonSerializer,
    };
    let (job, spec) = Cli::job::<GrpcDescriptor, _, _>().await?;
    let measure = MeasureLayer::new();
    let make = MakeChannel(measure.clone());
    let report = job.shot::<_, _, DynamicContract<serde_json::Value, JsonSerializer>>(make, &spec).await?;
    dbg!(measure.aggregated().times());
    Ok((!report.pass() as u8).into())
}

#[cfg(not(feature = "cli"))]
pub fn main() -> Result<ExitCode, Box<dyn std::error::Error + Send + Sync>> {
    Err("cli feature is not enabled".into())
}
