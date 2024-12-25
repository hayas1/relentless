#![cfg(all(feature = "json", feature = "yaml", feature = "console-report"))]
use axum::{body::Body, http::Request};
use http_body_util::BodyExt;
use relentless::interface::{
    command::{Relentless, WorkerKind},
    report::console::{CaseConsoleReport, RelentlessConsoleReport},
};

use relentless_dev_server::route::{self, counter::CounterResponse};
use tower::ServiceExt;

#[tokio::test]
async fn test_repeat_config() {
    let relentless = Relentless {
        file: vec!["tests/config/feature/repeat.yaml".into()],
        no_color: true,
        no_async_testcases: true,
        ..Default::default()
    };
    let configs = relentless.configs().unwrap();
    let service = route::app_with(Default::default());
    let report = relentless.assault_with::<_, Request<Body>>(configs, service.clone()).await.unwrap();

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
    assert!(relentless.pass(&report));
    assert!(relentless.allow(&report));

    let response = service.oneshot(Request::builder().uri("/counter").body(Body::empty()).unwrap()).await.unwrap();
    let count: CounterResponse<u64> = serde_json::from_slice(&response.collect().await.unwrap().to_bytes()).unwrap();
    assert_eq!(count.count, 90);
}

#[tokio::test]
async fn test_validate_config() {
    let relentless =
        Relentless { file: vec!["tests/config/feature/validate.yaml".into()], no_color: true, ..Default::default() };
    let configs = relentless.configs().unwrap();
    let service = route::app_with(Default::default());
    let report = relentless.assault_with::<_, Request<Body>>(configs, service).await.unwrap();

    let mut buf = Vec::new();
    relentless.report_with(&report, &mut buf).unwrap();
    let out = String::from_utf8_lossy(&buf);

    assert!(out.contains(&format!("{} /echo/json?foo=hoge&bar=fuga&baz=piyo", CaseConsoleReport::PASS_EMOJI)));
    assert!(relentless.pass(&report));
    assert!(relentless.allow(&report));
}

#[tokio::test]
async fn test_fail_validate_config() {
    let relentless = Relentless {
        file: vec!["tests/config/feature/fail_validate.yaml".into()],
        no_color: true,
        ..Default::default()
    };
    let configs = relentless.configs().unwrap();
    let service = route::app_with(Default::default());
    let report = relentless.assault_with::<_, Request<Body>>(configs, service).await.unwrap();

    let mut buf = Vec::new();
    relentless.report_with(&report, &mut buf).unwrap();
    let out = String::from_utf8_lossy(&buf);

    for line in [
        format!("{} /echo/json?foo=hoge&bar=fuga&baz=piyo", CaseConsoleReport::FAIL_EMOJI),
        format!("  {} message was found", CaseConsoleReport::MESSAGE_EMOJI),
        format!("    operation '{}' failed at path '{}': value did not match", "/0", ""),
    ] {
        assert!(out.contains(&line));
    }
    assert!(!relentless.pass(&report));
    assert!(!relentless.allow(&report));
}

#[tokio::test]
async fn test_allow_config() {
    let relentless =
        Relentless { file: vec!["tests/config/feature/allow.yaml".into()], no_color: true, ..Default::default() };
    let configs = relentless.configs().unwrap();
    let service = route::app_with(Default::default());
    let report = relentless.assault_with::<_, Request<Body>>(configs, service).await.unwrap();

    let mut buf = Vec::new();
    relentless.report_with(&report, &mut buf).unwrap();
    let out = String::from_utf8_lossy(&buf);

    for line in [
        format!("{} /health/disabled", CaseConsoleReport::FAIL_EMOJI),
        format!("  {} this testcase is allowed", CaseConsoleReport::ALLOW_EMOJI),
        format!("  {} message was found", CaseConsoleReport::MESSAGE_EMOJI),
        format!("    status{} is not acceptable", ""),
    ] {
        assert!(out.contains(&line));
    }
    assert!(!relentless.pass(&report));
    assert!(relentless.allow(&report));
}

#[tokio::test]
async fn test_json_diff_config() {
    let relentless =
        Relentless { file: vec!["tests/config/feature/json_diff.yaml".into()], no_color: true, ..Default::default() };
    let configs = relentless.configs().unwrap();
    let service = route::app_with(Default::default());
    let report = relentless.assault_with::<_, Request<Body>>(configs, service).await.unwrap();

    let mut buf = Vec::new();
    relentless.report_with(&report, &mut buf).unwrap();
    let out = String::from_utf8_lossy(&buf);

    for line in [
        format!("{} /information", CaseConsoleReport::FAIL_EMOJI),
        format!("  {} this testcase is allowed", CaseConsoleReport::ALLOW_EMOJI),
        format!("  {} message was found", CaseConsoleReport::MESSAGE_EMOJI),
        format!("    diff in {}", "`/uri`"),
    ] {
        assert!(out.contains(&line));
    }
    assert!(!relentless.pass(&report));
    assert!(relentless.allow(&report));
}

#[tokio::test]
async fn test_headers_config() {
    let relentless =
        Relentless { file: vec!["tests/config/feature/headers.yaml".into()], no_color: true, ..Default::default() };
    let configs = relentless.configs().unwrap();
    let service = route::app_with(Default::default());
    let report = relentless.assault_with::<_, Request<Body>>(configs, service).await.unwrap();

    let mut buf = Vec::new();
    relentless.report_with(&report, &mut buf).unwrap();
    let out = String::from_utf8_lossy(&buf);

    assert!(out.contains(&format!("{} /echo/headers", CaseConsoleReport::PASS_EMOJI)));
    assert!(relentless.pass(&report));
    assert!(relentless.allow(&report));
}

#[tokio::test]
async fn test_fail_headers_config() {
    let relentless = Relentless {
        file: vec!["tests/config/feature/fail_validate_headers.yaml".into()],
        no_color: true,
        ..Default::default()
    };
    let configs = relentless.configs().unwrap();
    let service = route::app_with(Default::default());
    let report = relentless.assault_with::<_, Request<Body>>(configs, service).await.unwrap();

    let mut buf = Vec::new();
    relentless.report_with(&report, &mut buf).unwrap();
    let out = String::from_utf8_lossy(&buf);

    for line in [
        format!("{} /echo/headers", CaseConsoleReport::FAIL_EMOJI),
        format!("  {} message was found", CaseConsoleReport::MESSAGE_EMOJI),
        format!("    operation '{}' failed at path '{}': value did not match", "/0", ""),
    ] {
        assert!(out.contains(&line));
    }
    assert!(!relentless.pass(&report));
    assert!(!relentless.allow(&report));
}

#[tokio::test]
async fn test_body_config() {
    let relentless =
        Relentless { file: vec!["tests/config/feature/body.yaml".into()], no_color: true, ..Default::default() };
    let configs = relentless.configs().unwrap();
    let service = route::app_with(Default::default());
    let report = relentless.assault_with::<_, Request<Body>>(configs, service).await.unwrap();

    let mut buf = Vec::new();
    relentless.report_with(&report, &mut buf).unwrap();
    let out = String::from_utf8_lossy(&buf);

    for line in [
        format!("{} /echo/body", CaseConsoleReport::PASS_EMOJI),
        format!("{} /echo/json", CaseConsoleReport::PASS_EMOJI),
        format!(
            "{} /echo/json {} {}",
            CaseConsoleReport::PASS_EMOJI,
            CaseConsoleReport::DESCRIPTION_EMOJI,
            "json without Content-Type will return 415 Unsupported Media Type"
        ),
    ] {
        assert!(out.contains(&line));
    }
    assert!(relentless.pass(&report));
    assert!(relentless.allow(&report));
}

#[tokio::test]
async fn test_timeout_config() {
    let relentless =
        Relentless { file: vec!["tests/config/feature/timeout.yaml".into()], no_color: true, ..Default::default() };
    let configs = relentless.configs().unwrap();
    let service = route::app_with(Default::default());
    let report = relentless.assault_with::<_, Request<Body>>(configs, service).await.unwrap();

    let mut buf = Vec::new();
    relentless.report_with(&report, &mut buf).unwrap();
    let out = String::from_utf8_lossy(&buf);

    for line in [
        format!("{} /wait/1/s", CaseConsoleReport::FAIL_EMOJI),
        format!("  {} this testcase is allowed", CaseConsoleReport::ALLOW_EMOJI),
        format!("  {} message was found", CaseConsoleReport::MESSAGE_EMOJI),
        format!("    request timeout: {}", ""), // TODO regex
        format!("{} /wait/500/ms", CaseConsoleReport::PASS_EMOJI),
    ] {
        assert!(out.contains(&line));
    }
    assert!(!relentless.pass(&report));
    assert!(relentless.allow(&report));
}

#[tokio::test]
async fn test_template_config() {
    let relentless =
        Relentless { file: vec!["tests/config/feature/template.yaml".into()], no_color: true, ..Default::default() };
    let configs = relentless.configs().unwrap();
    let service = route::app_with(Default::default());
    let report = relentless.assault_with::<_, Request<Body>>(configs, service).await.unwrap();

    let mut buf = Vec::new();
    relentless.report_with(&report, &mut buf).unwrap();
    let out = String::from_utf8_lossy(&buf);

    for line in [
        format!("{} /echo/path/${{var}}", CaseConsoleReport::FAIL_EMOJI),
        format!("  {} this testcase is allowed", CaseConsoleReport::ALLOW_EMOJI),
        format!("{} /echo/body", CaseConsoleReport::PASS_EMOJI),
    ] {
        assert!(out.contains(&line));
    }
    assert!(!relentless.pass(&report));
    assert!(relentless.allow(&report));
}

#[tokio::test]
async fn test_async_config() {
    let relentless =
        Relentless { file: vec!["tests/config/feature/async.yaml".into(); 5], no_color: true, ..Default::default() };
    let configs = relentless.configs().unwrap();
    let service = route::app_with(Default::default());
    let report = relentless.assault_with::<_, Request<Body>>(configs, service).await.unwrap();

    let mut buf = Vec::new();
    relentless.report_with(&report, &mut buf).unwrap();
    let out = String::from_utf8_lossy(&buf);

    // TODO test for 5 times
    for line in [
        // 1
        format!("{} /wait/500/ns {}1000/1000 ", CaseConsoleReport::PASS_EMOJI, CaseConsoleReport::REPEAT_EMOJI),
        format!("{} /wait/500/us {}100/100 ", CaseConsoleReport::PASS_EMOJI, CaseConsoleReport::REPEAT_EMOJI),
        format!("{} /wait/500/ms {}10/10 ", CaseConsoleReport::PASS_EMOJI, CaseConsoleReport::REPEAT_EMOJI),
        format!("{} /wait/1/s {}5/5 ", CaseConsoleReport::PASS_EMOJI, CaseConsoleReport::REPEAT_EMOJI),
        // 2
        format!("{} /wait/500/ns {}1000/1000 ", CaseConsoleReport::PASS_EMOJI, CaseConsoleReport::REPEAT_EMOJI),
        format!("{} /wait/500/us {}100/100 ", CaseConsoleReport::PASS_EMOJI, CaseConsoleReport::REPEAT_EMOJI),
        format!("{} /wait/500/ms {}10/10 ", CaseConsoleReport::PASS_EMOJI, CaseConsoleReport::REPEAT_EMOJI),
        format!("{} /wait/1/s {}5/5 ", CaseConsoleReport::PASS_EMOJI, CaseConsoleReport::REPEAT_EMOJI),
        // 3
        format!("{} /wait/500/ns {}1000/1000 ", CaseConsoleReport::PASS_EMOJI, CaseConsoleReport::REPEAT_EMOJI),
        format!("{} /wait/500/us {}100/100 ", CaseConsoleReport::PASS_EMOJI, CaseConsoleReport::REPEAT_EMOJI),
        format!("{} /wait/500/ms {}10/10 ", CaseConsoleReport::PASS_EMOJI, CaseConsoleReport::REPEAT_EMOJI),
        format!("{} /wait/1/s {}5/5 ", CaseConsoleReport::PASS_EMOJI, CaseConsoleReport::REPEAT_EMOJI),
        // 4
        format!("{} /wait/500/ns {}1000/1000 ", CaseConsoleReport::PASS_EMOJI, CaseConsoleReport::REPEAT_EMOJI),
        format!("{} /wait/500/us {}100/100 ", CaseConsoleReport::PASS_EMOJI, CaseConsoleReport::REPEAT_EMOJI),
        format!("{} /wait/500/ms {}10/10 ", CaseConsoleReport::PASS_EMOJI, CaseConsoleReport::REPEAT_EMOJI),
        format!("{} /wait/1/s {}5/5 ", CaseConsoleReport::PASS_EMOJI, CaseConsoleReport::REPEAT_EMOJI),
        // 5
        format!("{} /wait/500/ns {}1000/1000 ", CaseConsoleReport::PASS_EMOJI, CaseConsoleReport::REPEAT_EMOJI),
        format!("{} /wait/500/us {}100/100 ", CaseConsoleReport::PASS_EMOJI, CaseConsoleReport::REPEAT_EMOJI),
        format!("{} /wait/500/ms {}10/10 ", CaseConsoleReport::PASS_EMOJI, CaseConsoleReport::REPEAT_EMOJI),
        format!("{} /wait/1/s {}5/5 ", CaseConsoleReport::PASS_EMOJI, CaseConsoleReport::REPEAT_EMOJI),
    ] {
        assert!(out.contains(&line));
    }
    assert!(relentless.pass(&report));
    assert!(relentless.allow(&report));
}

#[tokio::test]
async fn test_measure_config() {
    let relentless = Relentless {
        file: vec!["tests/config/feature/measure.yaml".into(); 3],
        no_color: true,
        measure: Some(vec![WorkerKind::Repeats, WorkerKind::Testcases, WorkerKind::Configs]),
        percentile: Some(vec![5., 50., 90., 95., 99., 99.9]),
        ..Default::default()
    };
    let configs = relentless.configs().unwrap();
    let service = route::app_with(Default::default());
    let report = relentless.assault_with::<_, Request<Body>>(configs, service).await.unwrap();

    let mut buf = Vec::new();
    relentless.report_with(&report, &mut buf).unwrap();
    let out = String::from_utf8_lossy(&buf);

    // TODO test for 3 times
    for line in [
        // 1
        format!("  {} /health {}100/100 ", CaseConsoleReport::PASS_EMOJI, CaseConsoleReport::REPEAT_EMOJI),
        format!("    {} summery of all requests in repeats", RelentlessConsoleReport::SUMMARY_EMOJI),
        format!("      pass-rt: 100/100={:.2}%    rps: 100req/", 100.),
        format!("      latency: min={}", ""),
        format!("  {} /health {}1000/1000 ", CaseConsoleReport::PASS_EMOJI, CaseConsoleReport::REPEAT_EMOJI),
        format!("      pass-rt: 1000/1000={:.2}%    rps: 1000req/", 100.),
        format!("      latency: min={}", ""),
        format!("  {} /health {}10000/10000 ", CaseConsoleReport::PASS_EMOJI, CaseConsoleReport::REPEAT_EMOJI),
        format!("      pass-rt: 10000/10000={:.2}%    rps: 10000req/", 100.),
        format!("      latency: min={}", ""),
        format!("  {} summery of all requests in testcases", RelentlessConsoleReport::SUMMARY_EMOJI),
        format!("    pass-rt: 11100/11100={:.2}%    rps: 11100req/", 100.),
        format!("    latency: min={}", ""),
        // 2
        format!("  {} /health {}100/100 ", CaseConsoleReport::PASS_EMOJI, CaseConsoleReport::REPEAT_EMOJI),
        format!("    {} summery of all requests in repeats", RelentlessConsoleReport::SUMMARY_EMOJI),
        format!("      pass-rt: 100/100={:.2}%    rps: 100req/", 100.),
        format!(" p5={}", ""),
        format!("  {} /health {}1000/1000 ", CaseConsoleReport::PASS_EMOJI, CaseConsoleReport::REPEAT_EMOJI),
        format!("      pass-rt: 1000/1000={:.2}%    rps: 1000req/", 100.),
        format!(" p5={}", ""),
        format!("  {} /health {}10000/10000 ", CaseConsoleReport::PASS_EMOJI, CaseConsoleReport::REPEAT_EMOJI),
        format!("      pass-rt: 10000/10000={:.2}%    rps: 10000req/", 100.),
        format!(" p5={}", ""),
        format!("  {} summery of all requests in testcases", RelentlessConsoleReport::SUMMARY_EMOJI),
        format!("    pass-rt: 11100/11100={:.2}%    rps: 11100req/", 100.),
        format!(" p5={}", ""),
        // 3
        format!("  {} /health {}100/100 ", CaseConsoleReport::PASS_EMOJI, CaseConsoleReport::REPEAT_EMOJI),
        format!("    {} summery of all requests in repeats", RelentlessConsoleReport::SUMMARY_EMOJI),
        format!("      pass-rt: 100/100={:.2}%    rps: 100req/", 100.),
        format!(" p99.9={}", ""),
        format!("  {} /health {}1000/1000 ", CaseConsoleReport::PASS_EMOJI, CaseConsoleReport::REPEAT_EMOJI),
        format!("      pass-rt: 1000/1000={:.2}%    rps: 1000req/", 100.),
        format!(" p99.9={}", ""),
        format!("  {} /health {}10000/10000 ", CaseConsoleReport::PASS_EMOJI, CaseConsoleReport::REPEAT_EMOJI),
        format!("      pass-rt: 10000/10000={:.2}%    rps: 10000req/", 100.),
        format!(" p99.9={}", ""),
        format!("  {} summery of all requests in testcases", RelentlessConsoleReport::SUMMARY_EMOJI),
        format!("    pass-rt: 11100/11100={:.2}%    rps: 11100req/", 100.),
        format!(" p99.9={}", ""),
        // summery
        format!(
            "{} summery of all requests in configs {}",
            RelentlessConsoleReport::SUMMARY_EMOJI,
            RelentlessConsoleReport::SUMMARY_EMOJI,
        ),
        format!("  pass-rt: 33300/33300={:.2}%    rps: 33300req/", 100.),
        format!("  latency: min={}", ""),
    ] {
        assert!(out.contains(&line));
    }
    assert!(relentless.pass(&report));
    assert!(relentless.allow(&report));
    println!("{}", out);
}
