#![cfg(all(feature = "json", feature = "yaml"))]
use axum::{body::Body, http::Request};
use relentless::interface::command::Relentless;
use relentless_dev_server_http::route;

#[tokio::test]
async fn test_record_config() {
    let relentless = Relentless {
        file: vec!["tests/config/record/config.yaml".into()],
        no_color: true,
        output_record: Some("tests/record_test_directory".into()),
        ..Default::default()
    };
    let (configs, _) = relentless.configs();
    let service = route::app_with(Default::default());
    let record_service = relentless.build_service::<_, Request<Body>>(service);
    let report = relentless.assault_with::<_, Request<Body>>(configs, record_service).await.unwrap();
    assert!(relentless.pass(&report));
    assert!(relentless.allow(&report));

    let gitignore_path = "tests/record_test_directory/.gitignore";
    let gitignore_content = std::fs::read_to_string(gitignore_path).unwrap();
    assert_eq!(gitignore_content, "*\n");

    let raw_request_path = "tests/record_test_directory/http:/localhost:3000/echo/body/raw_request.txt";
    let raw_request_record = std::fs::read_to_string(raw_request_path).unwrap();
    assert_eq!(
        raw_request_record,
        indoc::indoc! {
          r#"POST http://localhost:3000/echo/body HTTP/1.1
            content-type: "text/plain"
            content-length: "11"

            hello world"#
        }
    );

    let raw_response_path = "tests/record_test_directory/http:/localhost:3000/echo/body/raw_response.txt";
    let raw_response_record = std::fs::read_to_string(raw_response_path).unwrap();
    assert_eq!(
        raw_response_record,
        indoc::indoc! {
          r#"HTTP/1.1 200 OK
            content-type: "application/octet-stream"
            content-length: "11"

            hello world"#
        }
    );

    let request_path = "tests/record_test_directory/http:/localhost:3000/health/rich/request.txt";
    let request_record = std::fs::read_to_string(request_path).unwrap();
    assert_eq!(request_record, "");

    let response_path = "tests/record_test_directory/http:/localhost:3000/health/rich/response.json";
    let response_record = std::fs::read_to_string(response_path).unwrap();
    assert_eq!(response_record, r#"{"status":"200 OK","code":200}"#);
}
