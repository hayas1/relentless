#![cfg(all(feature = "json", feature = "yaml"))]
use axum::{body::Body, http::Request};
use relentless::interface::{
    command::{Relentless, ReportFormat},
    report::github_markdown::CaseGithubMarkdownReport,
};
use relentless_dev_server::route;

#[tokio::test]
async fn test_github_markdown_report_format() {
    let relentless = Relentless {
        file: vec!["tests/config/feature/allow.yaml".into()],
        report_format: ReportFormat::GithubMarkdown,
        ..Default::default()
    };
    let configs = relentless.configs().unwrap();
    let service = route::app_with(Default::default());
    let report = relentless.assault_with::<_, Request<Body>>(configs, service).await.unwrap();

    let mut buf = Vec::new();
    relentless.report_with(&report, &mut buf).unwrap();
    let out = String::from_utf8_lossy(&buf);

    for line in [
        format!("{} `/health/disabled`", CaseGithubMarkdownReport::FAIL_EMOJI),
        format!("  {} this testcase is allowed", CaseGithubMarkdownReport::ALLOW_EMOJI),
        format!("  <details>{}", ""),
        format!("    <summary> {} message was found </summary>", CaseGithubMarkdownReport::MESSAGE_EMOJI),
        format!("    ```{}", ""),
        format!("    status{} is not acceptable", ""),
        format!("    ```{}", ""),
        format!("  </details>{}", ""),
    ] {
        assert!(out.contains(&line));
    }
    assert!(!relentless.pass(&report));
    assert!(relentless.allow(&report));
}
