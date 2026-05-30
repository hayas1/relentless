use init_tracing_opentelemetry::{resource::DetectResource, TracingConfig};
use relentless_grpc_dev_server::runner::RunCommand;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let _guard = TracingConfig::development()
        .with_resource_config(
            DetectResource::default()
                .with_fallback_service_name(env!("CARGO_PKG_NAME"))
                .with_fallback_service_version(env!("CARGO_PKG_VERSION")),
        )
        .init_subscriber()?;
    let rc = RunCommand::cli();
    rc.serve().await?;
    Ok(())
}
