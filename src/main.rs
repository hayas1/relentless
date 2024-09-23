use std::process::ExitCode;

use relentless::command::execute;

#[tokio::main]
pub async fn main() -> Result<ExitCode, Box<dyn std::error::Error + Send + Sync>> {
    execute().await
}
