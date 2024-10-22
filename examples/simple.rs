use std::process::ExitCode;

use relentless::{command::Relentless, report::ConsoleReport};

#[tokio::main]
async fn main() -> Result<ExitCode, Box<dyn std::error::Error + Send + Sync>> {
    let cmd = Relentless {
        file: vec!["examples/config/assault.yaml".into(), "examples/config/compare.yaml".into()],
        ..Default::default()
    };

    let report = cmd.assault().await?;

    report.console_report_stdout(&cmd)?;
    Ok(report.exit_code(cmd))
}
