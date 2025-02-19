use std::marker::PhantomData;

use bytes::Bytes;
use http_body::Body;
use relentless::interface::command::{Assault, Relentless};
use serde::{Deserialize, Serialize};

use crate::{evaluate::HttpResponse, factory::HttpRequest, record::HttpIoRecorder};

#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct HttpAssault<ReqB, ResB> {
    relentless: Relentless,
    phantom: PhantomData<(ReqB, ResB)>,
}
impl<ReqB, ResB> Assault<http::Request<ReqB>, http::Response<ResB>> for HttpAssault<ReqB, ResB>
where
    ReqB: Body + From<Bytes> + Default,
    ResB: Body + From<Bytes> + Default,
    ResB::Error: std::error::Error + Send + Sync + 'static,
{
    type Request = HttpRequest;
    type Response = HttpResponse;
    type Recorder = HttpIoRecorder;
    type Layer = ();

    fn command(&self) -> &Relentless {
        &self.relentless
    }
    fn recorder(&self) -> Self::Recorder {
        HttpIoRecorder
    }
    fn layer(&self) -> Self::Layer {}
}

impl<ReqB, ResB> HttpAssault<ReqB, ResB> {
    pub fn new(relentless: Relentless) -> Self {
        Self { relentless, phantom: PhantomData }
    }
}

#[cfg(test)]
#[cfg(all(feature = "yaml", feature = "json"))]
mod tests {
    use relentless::{
        assault::{
            destinations::{AllOr, Destinations},
            evaluator::json::JsonEvaluator,
        },
        interface::config::{Config, Format, Setting, Severity, Testcase, WorkerConfig},
    };

    use crate::{
        evaluate::{HttpBody, HttpHeaders, HttpResponse},
        factory::HttpRequest,
    };

    #[test]
    fn test_config_roundtrip() {
        let example = Config {
            worker_config: WorkerConfig {
                name: Some("example".to_string()),
                setting: Setting {
                    request: HttpRequest::default(),
                    response: HttpResponse { header: HttpHeaders::Ignore, ..Default::default() },
                    ..Default::default()
                },
                ..Default::default()
            },
            testcases: vec![Testcase {
                description: Some("test description".to_string()),
                target: "/information".to_string(),
                setting: Setting {
                    request: HttpRequest::default(),
                    allow: Some(true),
                    response: HttpResponse {
                        body: HttpBody::Json(JsonEvaluator {
                            ignore: vec!["/datetime".to_string()],
                            // patch: Some(PatchTo::All(
                            //     serde_json::from_value(
                            //         serde_json::json!([{"op": "replace", "path": "/datetime", "value": "2021-01-01"}]),
                            //     )
                            //     .unwrap(),
                            // )),
                            patch: Some(AllOr::Destinations(Destinations::from_iter([
                                (
                                    "actual".to_string(),
                                    serde_json::from_value(serde_json::json!([{"op": "remove", "path": "/datetime"}]))
                                        .unwrap(),
                                ),
                                (
                                    "expect".to_string(),
                                    serde_json::from_value(serde_json::json!([{"op": "remove", "path": "/datetime"}]))
                                        .unwrap(),
                                ),
                            ]))),
                            patch_fail: Some(Severity::Deny),
                        }),
                        ..Default::default()
                    },
                    ..Default::default()
                },
            }],
        };
        let yaml = serde_yaml::to_string(&example).unwrap();
        // println!("{}", yaml);

        let round_trip = Config::read_str(&yaml, Format::Yaml).unwrap();
        assert_eq!(example, round_trip);
    }

    #[test]
    fn test_config_json_patch() {
        let all_yaml = r#"
        name: json patch to all
        destinations:
          actual: http://localhost:3000
          expect: http://localhost:3000
        testcases:
        - description: test description
          target: /information
          setting:
            response:
              body:
                json:
                  patch:
                  - op: replace
                    path: /datetime
                    value: 2021-01-01
        "#;
        let config = Config::read_str(all_yaml, Format::Yaml).unwrap();
        assert_eq!(
            config.testcases[0].setting,
            Setting {
                request: HttpRequest::default(),
                response: HttpResponse {
                    body: HttpBody::Json(JsonEvaluator {
                        ignore: vec![],
                        patch: Some(AllOr::All(
                            serde_json::from_value(
                                serde_json::json!([{"op": "replace", "path": "/datetime", "value": "2021-01-01"}])
                            )
                            .unwrap(),
                        )),
                        patch_fail: None,
                    }),
                    ..Default::default()
                },
                ..Default::default()
            },
        );

        let destinations_yaml = r#"
        name: json patch to destinations
        destinations:
          actual: http://localhost:3000
          expect: http://localhost:3000
        testcases:
        - description: test description
          target: /information
          setting:
            response:
              body:
                json:
                  patch:
                    actual:
                    - op: remove
                      path: /datetime
                    expect:
                    - op: remove
                      path: /datetime
                  patch-fail: warn
        "#;
        let config = Config::read_str(destinations_yaml, Format::Yaml).unwrap();
        assert_eq!(
            config.testcases[0].setting,
            Setting {
                request: HttpRequest::default(),
                response: HttpResponse {
                    body: HttpBody::Json(JsonEvaluator {
                        ignore: vec![],
                        patch: Some(AllOr::Destinations(Destinations::from_iter([
                            (
                                "actual".to_string(),
                                serde_json::from_value(serde_json::json!([{"op": "remove", "path": "/datetime"}]))
                                    .unwrap(),
                            ),
                            (
                                "expect".to_string(),
                                serde_json::from_value(serde_json::json!([{"op": "remove", "path": "/datetime"}]))
                                    .unwrap(),
                            ),
                        ]))),
                        patch_fail: Some(Severity::Warn),
                    }),
                    ..Default::default()
                },
                ..Default::default()
            },
        );
    }
}
