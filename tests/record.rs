#![cfg(all(feature = "json", feature = "yaml"))]
use axum::{body::Body, http::Request};
use relentless::{command::Relentless, evaluate::DefaultEvaluator};
use relentless_dev_server::route;

#[tokio::test]
async fn test_record_config() {
    let relentless = Relentless {
        file: vec!["tests/config/record/config.yaml".into()],
        no_color: true,
        output_record: Some("tests/record_test_directory".into()),
        ..Default::default()
    };
    let configs = relentless.configs().unwrap();
    let service = route::app_with(Default::default());
    let mut record_service = relentless.build_service::<_, Request<Body>>(service);
    let report =
        relentless.assault_with::<_, Request<Body>, _>(configs, &mut record_service, &DefaultEvaluator).await.unwrap();
    assert!(relentless.pass(&report));
    assert!(relentless.allow(&report));

    let gitignore_path = "tests/record_test_directory/.gitignore";
    let gitignore_content = std::fs::read_to_string(gitignore_path).unwrap();
    assert_eq!(gitignore_content, "*\n");

    let request_path = "tests/record_test_directory/http:/localhost:3000/echo/body/request.txt";
    let request_record = std::fs::read_to_string(request_path).unwrap();
    assert_eq!(
        request_record,
        indoc::indoc! {
          r#"POST http://localhost:3000/echo/body HTTP/1.1
            content-type: "text/plain"
            content-length: "11"

            hello world"#
        }
    );

    let response_path = "tests/record_test_directory/http:/localhost:3000/echo/body/response.txt";
    let response_record = std::fs::read_to_string(response_path).unwrap();
    assert_eq!(
        response_record,
        indoc::indoc! {
          r#"HTTP/1.1 200 OK
            content-type: "application/octet-stream"
            content-length: "11"

            hello world"#
        }
    );
}
