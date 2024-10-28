#![cfg(all(feature = "json", feature = "yaml", feature = "console-report"))]
use axum::{body::Body, http::Request};
use http_body_util::BodyExt;
use relentless::{
    command::Relentless,
    evaluate::DefaultEvaluator,
    report::{console_report::CaseConsoleReport, Reportable},
};

use relentless_dev_server::route::{self, counter::CounterResponse};
use tower::Service;

#[tokio::test]
async fn test_repeat_config() {
    let relentless =
        Relentless { file: vec!["tests/config/feature/repeat.yaml".into()], no_color: true, ..Default::default() };
    let configs = relentless.configs().unwrap();
    let mut services = vec![[("test-api".to_string(), route::app_with(Default::default()))].into_iter().collect()];
    let report = relentless.assault_with::<_, Body, Body, _>(configs, &mut services, &DefaultEvaluator).await.unwrap();

    let mut buf = Vec::new();
    relentless.report_with(&report, &mut buf).unwrap();
    let out = String::from_utf8_lossy(&buf);

    for line in [
        format!("{} /counter/increment {}10/10", CaseConsoleReport::PASS_EMOJI, CaseConsoleReport::REPEAT_EMOJI),
        format!("{} /counter/increment/10 {}10/10", CaseConsoleReport::PASS_EMOJI, CaseConsoleReport::REPEAT_EMOJI),
        format!("{} /counter/decrement {}10/10", CaseConsoleReport::PASS_EMOJI, CaseConsoleReport::REPEAT_EMOJI),
        format!("{} /counter/decrement/10 {}1/1", CaseConsoleReport::PASS_EMOJI, CaseConsoleReport::REPEAT_EMOJI),
    ] {
        assert!(out.contains(&line));
    }
    assert!(report.pass());

    let response = services[0]
        .get_mut("test-api")
        .unwrap()
        .call(Request::builder().uri("/counter").body(Body::empty()).unwrap())
        .await
        .unwrap();
    let count: CounterResponse<u64> = serde_json::from_slice(&response.collect().await.unwrap().to_bytes()).unwrap();
    assert_eq!(count.count, 90);
}
