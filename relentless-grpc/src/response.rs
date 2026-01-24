use std::fmt::Debug;

use relentless::{
    error::EvaluateError,
    evaluator::{
        evaluate::{Evaluator, Failure, Messages},
        json::JsonEvaluator,
        plaintext::RegexEvaluator,
    },
    shot::{contract::ResponseSink, destinations::Destinations},
};
use semigroup::Semigroup;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize, Semigroup)]
#[serde(deny_unknown_fields, rename_all = "kebab-case")]
#[semigroup(with = "semigroup::op::Coalesce")]
pub struct GrpcResponse {
    #[cfg_attr(feature = "yaml", serde(with = "serde_yaml::with::singleton_map_recursive"))]
    #[serde(default)]
    pub status: Option<GrpcResponseStatus>,
    #[cfg_attr(feature = "yaml", serde(with = "serde_yaml::with::singleton_map_recursive"))]
    #[serde(default)]
    pub metadata_map: Option<GrpcResponseMetadataMap>,
    #[cfg_attr(feature = "yaml", serde(with = "serde_yaml::with::singleton_map_recursive"))]
    #[serde(default)]
    pub message: Option<GrpcResponseMessage>,
}
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "kebab-case")]
pub enum GrpcResponseStatus {
    #[default]
    OkOrEqual,
    // Expect(ExpectEvaluator<tonic::Code>), // TODO serde
    Ignore,
}
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "kebab-case")]
pub enum GrpcResponseMetadataMap {
    #[default]
    AnyOrEqual,
    // Expect(ExpectEvaluator<tonic::metadata::MetadataMap>), // TODO serde
    Ignore,
}
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "kebab-case")]
pub enum GrpcResponseMessage {
    #[default]
    AnyOrEqual,
    Plaintext(RegexEvaluator),
    Json(JsonEvaluator),
}

impl<Se: Debug + Send> ResponseSink<Result<tonic::Response<Se>, tonic::Status>> for GrpcResponse {
    type Message = EvaluateError;
    #[tracing::instrument(err)]
    async fn consume(
        &self,
        msg: &mut Messages<Self::Message>,
        res: Destinations<Result<tonic::Response<Se>, tonic::Status>>,
    ) -> Result<(), Failure> {
        self.evaluate(msg, res)
    }
}

impl<Se> Evaluator<Result<tonic::Response<Se>, tonic::Status>> for GrpcResponse {
    type Message = EvaluateError;
    fn evaluate_shot(
        &self,
        msg: &mut Messages<Self::Message>,
        res: &Result<tonic::Response<Se>, tonic::Status>,
    ) -> Result<(), Failure> {
        self.status.as_ref().unwrap_or(&Default::default()).evaluate_shot(msg, res)?;
        Ok(())
    }
    fn evaluate_compare(
        &self,
        msg: &mut Messages<Self::Message>,
        res1: &Result<tonic::Response<Se>, tonic::Status>,
        res2: &Result<tonic::Response<Se>, tonic::Status>,
    ) -> Result<(), Failure> {
        self.status.as_ref().unwrap_or(&Default::default()).evaluate_compare(msg, res1, res2)?;
        Ok(())
    }
}

impl<Se> Evaluator<Result<tonic::Response<Se>, tonic::Status>> for GrpcResponseStatus {
    type Message = EvaluateError;
    fn evaluate_shot(
        &self,
        msg: &mut Messages<Self::Message>,
        res: &Result<tonic::Response<Se>, tonic::Status>,
    ) -> Result<(), Failure> {
        match self {
            Self::OkOrEqual => <Self as Evaluator<Result<tonic::Response<Se>, tonic::Status>>>::evaluate_bool(
                self,
                msg,
                res.is_ok(),
                |_| EvaluateError::custom("not success status"),
            ),
            Self::Ignore => Ok(()),
        }
    }
    fn evaluate_compare(
        &self,
        msg: &mut Messages<Self::Message>,
        res1: &Result<tonic::Response<Se>, tonic::Status>,
        res2: &Result<tonic::Response<Se>, tonic::Status>,
    ) -> Result<(), Failure> {
        match self {
            Self::OkOrEqual => <Self as Evaluator<Result<tonic::Response<Se>, tonic::Status>>>::evaluate_bool(
                self,
                msg,
                res1.is_ok() == res2.is_ok() || res1.is_err() == res2.is_err(),
                |_| EvaluateError::custom("not equal status"),
            ),
            Self::Ignore => Ok(()),
        }
    }
}
