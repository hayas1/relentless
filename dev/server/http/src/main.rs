use relentless_http_dev_server::runner::RunCommand;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    tracing_subscriber::fmt::init();
    let rc = RunCommand::cli();
    rc.serve().await
}
