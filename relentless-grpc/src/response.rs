use std::fmt::Debug;

use relentless::{
    error::EvaluateError,
    evaluator::{
        evaluate::{Evaluator, Failure, Messages},
        json::JsonEvaluator,
    },
    shot::{contract::ResponseSink, destinations::Destinations},
};
use semigroup::Semigroup;
use serde::{Deserialize, Serialize};
use tonic::metadata::MetadataMap;

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
    Value(JsonEvaluator),
}

impl<Se: Debug + Send + PartialEq> ResponseSink<Result<tonic::Response<Se>, tonic::Status>> for GrpcResponse {
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

impl<Se: PartialEq> Evaluator<Result<tonic::Response<Se>, tonic::Status>> for GrpcResponse {
    type Message = EvaluateError;
    fn evaluate_shot(
        &self,
        msg: &mut Messages<Self::Message>,
        res: &Result<tonic::Response<Se>, tonic::Status>,
    ) -> Result<(), Failure> {
        self.status.as_ref().unwrap_or(&Default::default()).evaluate_shot(msg, res)?;
        let resp = res.as_ref().map_err(|_| Failure)?;
        self.metadata_map.as_ref().unwrap_or(&Default::default()).evaluate_shot(msg, resp.metadata())?;
        self.message.as_ref().unwrap_or(&Default::default()).evaluate_shot(msg, resp.get_ref())?;
        Ok(())
    }
    fn evaluate_compare(
        &self,
        msg: &mut Messages<Self::Message>,
        res1: &Result<tonic::Response<Se>, tonic::Status>,
        res2: &Result<tonic::Response<Se>, tonic::Status>,
    ) -> Result<(), Failure> {
        self.status.as_ref().unwrap_or(&Default::default()).evaluate_compare(msg, res1, res2)?;
        let resp1 = res1.as_ref().map_err(|_| Failure)?;
        let resp2 = res2.as_ref().map_err(|_| Failure)?;
        self.metadata_map.as_ref().unwrap_or(&Default::default()).evaluate_compare(
            msg,
            resp1.metadata(),
            resp2.metadata(),
        )?;
        self.message.as_ref().unwrap_or(&Default::default()).evaluate_compare(msg, resp1.get_ref(), resp2.get_ref())?;
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

impl Evaluator<MetadataMap> for GrpcResponseMetadataMap {
    type Message = EvaluateError;
    fn evaluate_shot(&self, _msg: &mut Messages<Self::Message>, _res: &MetadataMap) -> Result<(), Failure> {
        match self {
            Self::AnyOrEqual => Ok(()),
            Self::Ignore => Ok(()),
        }
    }
    fn evaluate_compare(
        &self,
        msg: &mut Messages<Self::Message>,
        res1: &MetadataMap,
        res2: &MetadataMap,
    ) -> Result<(), Failure> {
        match self {
            // TODO use http impl ?
            Self::AnyOrEqual => {
                self.evaluate_bool(msg, res1.as_ref() == res2.as_ref(), |_| EvaluateError::custom("not equal metadata"))
            }
            Self::Ignore => Ok(()),
        }
    }
}

impl<Se: PartialEq> Evaluator<Se> for GrpcResponseMessage {
    type Message = EvaluateError;
    fn evaluate_shot(&self, _msg: &mut Messages<Self::Message>, _res: &Se) -> Result<(), Failure> {
        match self {
            Self::AnyOrEqual => Ok(()),
            Self::Value(_) => todo!(),
        }
    }
    fn evaluate_compare(&self, msg: &mut Messages<Self::Message>, res1: &Se, res2: &Se) -> Result<(), Failure> {
        match self {
            Self::AnyOrEqual => <Self as Evaluator<Se>>::evaluate_bool(self, msg, res1 == res2, |_| {
                EvaluateError::custom("not equal message")
            }),
            Self::Value(_) => todo!(),
        }
    }
}
