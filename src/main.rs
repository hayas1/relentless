use std::process::ExitCode;

use relentless::cli::run;
#[tokio::main]

pub async fn main() -> Result<ExitCode, Box<dyn std::error::Error + Send + Sync>> {
    run().await
}
