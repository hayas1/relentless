#![cfg(all(feature = "json", feature = "yaml", feature = "console-report"))]
use axum::body::Body;
use relentless::{
    command::{Relentless, ReportFormat},
    evaluate::DefaultEvaluator,
    report::{
        console_report::{ConsoleCaseReport, ConsoleReport, ReportWriter},
        Reportable,
    },
};

use relentless_dev_server::route;

#[tokio::test]
async fn test_repeat_config() {
    let relentless = Relentless {
        file: vec!["tests/config/feature/repeat.yaml".into()],
        report_format: ReportFormat::NullDevice,
        no_color: true,
        ..Default::default()
    };
    let configs = relentless.configs().unwrap();
    let test_api = route::app_with(Default::default());
    let services = [("test-api".to_string(), test_api)].into_iter().collect();
    let report = relentless.assault_with::<_, Body, Body, _>(configs, vec![services], &DefaultEvaluator).await.unwrap();

    let mut buf = Vec::new();
    report.console_report(&relentless, &mut ReportWriter::new(0, &mut buf)).unwrap();
    let out = String::from_utf8_lossy(&buf);

    for line in [
        format!("{} /counter/increment {}10/10", ConsoleCaseReport::PASS_EMOJI, ConsoleCaseReport::REPEAT_EMOJI),
        format!("{} /counter/increment/10 {}10/10", ConsoleCaseReport::PASS_EMOJI, ConsoleCaseReport::REPEAT_EMOJI),
        format!("{} /counter/decrement {}10/10", ConsoleCaseReport::PASS_EMOJI, ConsoleCaseReport::REPEAT_EMOJI),
        format!("{} /counter/decrement/10 {}1/1", ConsoleCaseReport::PASS_EMOJI, ConsoleCaseReport::REPEAT_EMOJI),
    ] {
        assert!(out.contains(&line));
    }
    assert!(report.pass());
}
