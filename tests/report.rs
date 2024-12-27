#![cfg(all(feature = "json", feature = "yaml"))]
use std::vec;

use axum::{body::Body, http::Request};
use relentless::interface::{
    command::{Relentless, ReportFormat, WorkerKind},
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
    let (configs, _) = relentless.configs();
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

#[tokio::test]
async fn test_github_markdown_measure() {
    let relentless = Relentless {
        file: vec!["tests/config/feature/measure.yaml".into()],
        report_format: ReportFormat::GithubMarkdown,
        measure: Some(vec![WorkerKind::Configs]),
        percentile: Some(vec![50., 99.]),
        ..Default::default()
    };
    let (configs, _) = relentless.configs();
    let service = route::app_with(Default::default());
    let report = relentless.assault_with::<_, Request<Body>>(configs, service).await.unwrap();

    let mut buf = Vec::new();
    relentless.report_with(&report, &mut buf).unwrap();
    let out = String::from_utf8_lossy(&buf);

    for line in [
        format!("{} `/health` {}100/100", CaseGithubMarkdownReport::PASS_EMOJI, CaseGithubMarkdownReport::REPEAT_EMOJI),
        format!(
            "{} `/health` {}1000/1000",
            CaseGithubMarkdownReport::PASS_EMOJI,
            CaseGithubMarkdownReport::REPEAT_EMOJI
        ),
        format!(
            "{} `/health` {}10000/10000",
            CaseGithubMarkdownReport::PASS_EMOJI,
            CaseGithubMarkdownReport::REPEAT_EMOJI
        ),
        format!("{}| | min | mean | p50 | p99 | max |", ""),
        format!("{}| --- | --- | --- | --- | --- | --- |", ""),
        format!("{}| latency | ", ""),
        format!("{}pass rate: 11100/11100=100.00%, rps: 11100req/", ""),
    ] {
        assert!(out.contains(&line));
    }
    assert!(relentless.pass(&report));
    assert!(relentless.allow(&report));
}
