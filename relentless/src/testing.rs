use std::{
    convert::Infallible,
    fmt::Display,
    future::Future,
    pin::Pin,
    task::{Context, Poll},
};

use futures::{StreamExt, TryStreamExt};
use semigroup::Semigroup;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use tower::{layer::util::Identity, Layer, Service};

use crate::{
    error::EvaluateError,
    evaluator::{
        evaluate::{Evaluator, Failure, Messages},
        expect::ExpectEvaluator,
    },
    shot::{
        contract::{Contract, RequestSource, ResponseSink, SignContract},
        destinations::Destinations,
        job::BasePath,
    },
};

#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize, Semigroup)]
#[serde(deny_unknown_fields, rename_all = "kebab-case")]
#[semigroup(with = "semigroup::op::Coalesce")]
pub struct ValueRequest {
    pub value: Option<Value>,
}
impl RequestSource<Value> for ValueRequest {
    type Error = crate::Error;
    async fn produce(&self, _: &http::Uri, target: &str) -> Result<Value, Self::Error> {
        match target {
            "echo" => Ok(self.value.clone().unwrap_or_default()),
            "fail" => Err(crate::Error::custom("fail")),
            _ => Err(crate::Error::custom("unimplemented")),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize, Semigroup)]
#[serde(deny_unknown_fields, rename_all = "kebab-case")]
#[semigroup(with = "semigroup::op::Coalesce")]
pub struct ValueResponse {
    #[cfg_attr(feature = "yaml", serde(with = "serde_yaml::with::singleton_map_recursive"))]
    pub value: Option<ValueResponseInner>,
}
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "kebab-case")]
pub enum ValueResponseInner {
    #[default]
    AnyOrEqual,
    Expect(ExpectEvaluator<Value>),
    // Regex(RegexEvaluator), // TODO sized
}
impl<E: Display + Send> ResponseSink<Result<Value, E>> for ValueResponse {
    type Message = EvaluateError;
    async fn consume(
        &self,
        msg: &mut Messages<Self::Message>,
        res: Destinations<Result<Value, E>>,
    ) -> Result<(), Failure> {
        let buffers = res.len().max(1);
        let collected: Destinations<_> = futures::stream::iter(res)
            .map(|(d, r)| async { Ok((d, r.map_err(EvaluateError::custom)?)) })
            .buffer_unordered(buffers)
            .try_collect()
            .await
            .map_err(|e| msg.error(e))?;
        self.value.as_ref().unwrap_or(&Default::default()).evaluate(msg, collected)
    }
}
impl Evaluator<Value> for ValueResponseInner {
    type Message = EvaluateError;
    fn evaluate_shot(&self, msg: &mut Messages<Self::Message>, res: &Value) -> Result<(), Failure> {
        match self {
            Self::AnyOrEqual => Ok(()),
            Self::Expect(e) => e.evaluate_shot(msg, res),
        }
    }
    fn evaluate_compare(&self, msg: &mut Messages<Self::Message>, res1: &Value, res2: &Value) -> Result<(), Failure> {
        match self {
            Self::AnyOrEqual => self.evaluate_bool(msg, res1 == res2, |_| EvaluateError::custom("not equal body")),
            Self::Expect(e) => e.evaluate_compare(msg, res1, res2),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize, Semigroup)]
pub struct TestingClient;
impl Service<Value> for TestingClient {
    type Response = Value;
    type Error = Infallible;
    type Future = Pin<Box<dyn Future<Output = Result<Self::Response, Self::Error>> + Send>>;
    fn poll_ready(&mut self, _: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        Poll::Ready(Ok(()))
    }
    fn call(&mut self, req: Value) -> Self::Future {
        Box::pin(async move { Ok(req) })
    }
}
// MakeService
impl Service<http::Uri> for TestingClient {
    type Response = Self;
    type Error = Infallible;
    type Future = Pin<Box<dyn Future<Output = Result<Self::Response, Self::Error>> + Send>>;
    fn poll_ready(&mut self, _: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        Poll::Ready(Ok(()))
    }
    fn call(&mut self, _: http::Uri) -> Self::Future {
        Box::pin(async move { Ok(Self) })
    }
}
impl<S> Layer<S> for TestingClient {
    type Service = <Identity as Layer<S>>::Service;

    fn layer(&self, service: S) -> Self::Service {
        Identity::new().layer(service)
    }
}
impl Contract<Self> for TestingClient {
    type Sign = Self;
    type ReqSource = ValueRequest;
    type Request = Value;
    type TransportReq = Value;
    type TransportRes = Value;
    type Response = Value;
    type ResSink = ValueResponse;

    type SignError = Infallible;
}
impl SignContract<Self, Self> for TestingClient {
    type Error = Infallible;
    async fn sign_contract(&self, _: Self, _: &http::Uri, _: &Option<BasePath>) -> Result<Self, Self::Error> {
        Ok(TestingClient)
    }
}

#[cfg(test)]
mod tests {
    use crate::{
        report::{ReportFormat, Reporter},
        shot::{
            job::{Job, JobSpec},
            profile::Profile,
            suite::{Suite, SuiteCase},
            testcase::Testcase,
        },
    };

    use super::*;

    #[tokio::test]
    async fn test_compile_with_echo_service() {
        let (job, spec) = (Job(Vec::new()), JobSpec::default());
        let make = TestingClient;

        let report = job.shot::<TestingClient, TestingClient, TestingClient>(make, &spec).await.unwrap();
        spec.report(&report).unwrap();
        assert!(report.evaluated.assess().success());
    }

    #[tokio::test]
    async fn test_with_echo_service() {
        let suites = vec![SuiteCase {
            suite: Suite {
                name: "echo".to_string(),
                contract: Some(TestingClient),
                destinations: vec![("test", crate::http_newtype_serde::Uri("http://localhost:8080".parse().unwrap()))]
                    .into_iter()
                    .collect(),
                ..Default::default()
            },
            testcases: vec![
                Testcase {
                    target: "echo".to_string(),
                    profile: Profile {
                        request: ValueRequest { value: Some(serde_json::json!("hello")) },
                        response: ValueResponse {
                            value: Some(ValueResponseInner::Expect(ExpectEvaluator::new(serde_json::json!("hello")))),
                        },
                        ..Default::default()
                    },
                    ..Default::default()
                },
                Testcase {
                    target: "echo".to_string(),
                    profile: Profile {
                        request: ValueRequest { value: Some(serde_json::json!("value")) },
                        response: ValueResponse {
                            value: Some(ValueResponseInner::Expect(ExpectEvaluator::new(serde_json::json!("value")))),
                        },
                        ..Default::default()
                    },
                    ..Default::default()
                },
            ],
        }];
        let (job, spec) = (Job(suites), JobSpec { report_format: ReportFormat::NullDevice, ..Default::default() });
        let make = TestingClient;

        let report = job.shot::<TestingClient, TestingClient, TestingClient>(make, &spec).await.unwrap();
        spec.report(&report).unwrap();
        assert!(report.evaluated.assess().success());
    }
}
