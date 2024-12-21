#![cfg(all(feature = "json", feature = "yaml", feature = "console-report"))]
use axum::{body::Body, http::Request};
use http_body_util::BodyExt;
use relentless::interface::{command::Relentless, report::console::CaseConsoleReport};

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
        format!("{} /wait/500/ns {}1000/1000 ", CaseConsoleReport::PASS_EMOJI, CaseConsoleReport::REPEAT_EMOJI),
        format!("{} /wait/500/us {}100/100 ", CaseConsoleReport::PASS_EMOJI, CaseConsoleReport::REPEAT_EMOJI),
        format!("{} /wait/500/ms {}10/10 ", CaseConsoleReport::PASS_EMOJI, CaseConsoleReport::REPEAT_EMOJI),
        format!("{} /wait/1/s {}5/5 ", CaseConsoleReport::PASS_EMOJI, CaseConsoleReport::REPEAT_EMOJI),
        format!("{} /wait/500/ns {}1000/1000 ", CaseConsoleReport::PASS_EMOJI, CaseConsoleReport::REPEAT_EMOJI),
        format!("{} /wait/500/us {}100/100 ", CaseConsoleReport::PASS_EMOJI, CaseConsoleReport::REPEAT_EMOJI),
        format!("{} /wait/500/ms {}10/10 ", CaseConsoleReport::PASS_EMOJI, CaseConsoleReport::REPEAT_EMOJI),
        format!("{} /wait/1/s {}5/5 ", CaseConsoleReport::PASS_EMOJI, CaseConsoleReport::REPEAT_EMOJI),
        format!("{} /wait/500/ns {}1000/1000 ", CaseConsoleReport::PASS_EMOJI, CaseConsoleReport::REPEAT_EMOJI),
        format!("{} /wait/500/us {}100/100 ", CaseConsoleReport::PASS_EMOJI, CaseConsoleReport::REPEAT_EMOJI),
        format!("{} /wait/500/ms {}10/10 ", CaseConsoleReport::PASS_EMOJI, CaseConsoleReport::REPEAT_EMOJI),
        format!("{} /wait/1/s {}5/5 ", CaseConsoleReport::PASS_EMOJI, CaseConsoleReport::REPEAT_EMOJI),
        format!("{} /wait/500/ns {}1000/1000 ", CaseConsoleReport::PASS_EMOJI, CaseConsoleReport::REPEAT_EMOJI),
        format!("{} /wait/500/us {}100/100 ", CaseConsoleReport::PASS_EMOJI, CaseConsoleReport::REPEAT_EMOJI),
        format!("{} /wait/500/ms {}10/10 ", CaseConsoleReport::PASS_EMOJI, CaseConsoleReport::REPEAT_EMOJI),
        format!("{} /wait/1/s {}5/5 ", CaseConsoleReport::PASS_EMOJI, CaseConsoleReport::REPEAT_EMOJI),
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
