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
    },
};

#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize, Semigroup)]
pub struct StringRequest {
    #[semigroup(with = "semigroup::op::Coalesce")]
    pub message: Option<String>,
}
impl RequestSource<String> for StringRequest {
    type Error = Infallible;
    async fn produce(&self, _: &http::Uri, _: &str) -> Result<String, Self::Error> {
        Ok(self.message.clone().unwrap_or_default())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize, Semigroup)]
pub struct StringResponse {
    #[semigroup(with = "semigroup::op::Coalesce")]
    pub message: Option<StringResponseInner>,
}
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum StringResponseInner {
    #[default]
    AnyOrEqual,
    Expect(ExpectEvaluator<String>),
    // Regex(RegexEvaluator), // TODO sized
}
impl<E: Display + Send> ResponseSink<Result<String, E>> for StringResponse {
    type Message = EvaluateError;
    async fn consume(
        &self,
        msg: &mut Messages<Self::Message>,
        res: Destinations<Result<String, E>>,
    ) -> Result<(), Failure> {
        let buffers = res.len().max(1);
        let collected: Destinations<_> = futures::stream::iter(res)
            .map(|(d, r)| async { Ok((d, r.map_err(EvaluateError::custom)?)) })
            .buffer_unordered(buffers)
            .try_collect()
            .await
            .map_err(|e| msg.error(e))?;
        self.message.as_ref().unwrap_or(&Default::default()).evaluate(msg, collected)
    }
}
impl Evaluator<String> for StringResponseInner {
    type Message = EvaluateError;
    fn evaluate_shot(&self, msg: &mut Messages<Self::Message>, res: &String) -> Result<(), Failure> {
        match self {
            Self::AnyOrEqual => Ok(()),
            Self::Expect(e) => e.evaluate_shot(msg, res),
        }
    }
    fn evaluate_compare(&self, msg: &mut Messages<Self::Message>, res1: &String, res2: &String) -> Result<(), Failure> {
        match self {
            Self::AnyOrEqual => self.evaluate_bool(msg, res1 == res2, |_| EvaluateError::custom("not equal body")),
            Self::Expect(e) => e.evaluate_compare(msg, res1, res2),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize, Semigroup)]
pub struct EchoClient;
impl Service<String> for EchoClient {
    type Response = String;
    type Error = Infallible;
    type Future = Pin<Box<dyn Future<Output = Result<Self::Response, Self::Error>> + Send>>;
    fn poll_ready(&mut self, _: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        Poll::Ready(Ok(()))
    }
    fn call(&mut self, req: String) -> Self::Future {
        Box::pin(async move { Ok(req) })
    }
}
// MakeService
impl Service<http::Uri> for EchoClient {
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
impl<S> Layer<S> for EchoClient {
    type Service = <Identity as Layer<S>>::Service;

    fn layer(&self, service: S) -> Self::Service {
        Identity::new().layer(service)
    }
}
impl Contract<Self> for EchoClient {
    type Sign = Self;
    type ReqSource = StringRequest;
    type Request = String;
    type TransportReq = String;
    type TransportRes = String;
    type Response = String;
    type ResSink = StringResponse;

    type SignError = Infallible;
}
impl SignContract<Self, Self> for EchoClient {
    type Error = Infallible;
    async fn sign_contract(&self, _: Self, _: &http::Uri) -> Result<Self, Self::Error> {
        Ok(EchoClient)
    }
}

#[cfg(test)]
mod tests {
    use crate::{
        report::Reporter,
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
        let make = EchoClient;

        let report = job.shot::<EchoClient, EchoClient, EchoClient>(make, &spec).await.unwrap();
        spec.report(&report).unwrap();
        assert!(report.evaluated.assess().success());
    }

    #[tokio::test]
    async fn test_with_echo_service() {
        let suites = vec![SuiteCase {
            suite: Suite {
                name: "echo".to_string(),
                contract: Some(EchoClient),
                destinations: vec![("test", crate::http_newtype_serde::Uri("http://localhost:8080".parse().unwrap()))]
                    .into_iter()
                    .collect(),
                ..Default::default()
            },
            testcases: vec![
                Testcase {
                    target: String::new(),
                    profile: Profile {
                        request: StringRequest { message: Some("hello".to_string()) },
                        response: StringResponse {
                            message: Some(StringResponseInner::Expect(ExpectEvaluator::new("hello".to_string()))),
                        },
                        ..Default::default()
                    },
                    ..Default::default()
                },
                Testcase {
                    target: String::new(),
                    profile: Profile {
                        request: StringRequest { message: Some("world".to_string()) },
                        response: StringResponse {
                            message: Some(StringResponseInner::Expect(ExpectEvaluator::new("world".to_string()))),
                        },
                        ..Default::default()
                    },
                    ..Default::default()
                },
            ],
        }];
        let (job, spec) = (Job(suites), JobSpec::default());
        let make = EchoClient;

        let report = job.shot::<EchoClient, EchoClient, EchoClient>(make, &spec).await.unwrap();
        spec.report(&report).unwrap();
        assert!(report.evaluated.assess().success());
    }
}
