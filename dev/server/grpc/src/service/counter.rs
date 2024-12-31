use std::sync::{Arc, RwLock};

use num::{BigInt, ToPrimitive};
use relentless_dev_server_grpc_entity::counter_pb::{self, counter_server::Counter};
use thiserror::Error;
use tonic::{Request, Response, Status};

#[derive(Debug, Default)]
pub struct CounterImpl {
    pub counter: Arc<RwLock<BigInt>>,
}

#[tonic::async_trait]
impl Counter for CounterImpl {
    #[tracing::instrument(ret)]
    async fn increment(&self, request: Request<i64>) -> Result<Response<i64>, Status> {
        let value = BigInt::from(request.into_inner());
        let incremented = self.bigint_increment(value)?;
        Ok(Response::new(Self::cast_bigint(incremented)?))
    }
    #[tracing::instrument(ret)]
    async fn bincrement(&self, request: Request<counter_pb::BigInt>) -> Result<Response<counter_pb::BigInt>, Status> {
        let bint = request.into_inner().into();
        let incremented = self.bigint_increment(bint)?;
        Ok(Response::new(incremented.into()))
    }
    #[tracing::instrument(ret)]
    async fn decrement(&self, request: Request<i64>) -> Result<Response<i64>, Status> {
        let value = request.into_inner();
        let decremented = self.bigint_increment(BigInt::from(-value))?;
        Ok(Response::new(Self::cast_bigint(decremented)?))
    }
    #[tracing::instrument(ret)]
    async fn bdecrement(&self, request: Request<counter_pb::BigInt>) -> Result<Response<counter_pb::BigInt>, Status> {
        let bint = BigInt::from(request.into_inner());
        let decremented = self.bigint_increment(-bint)?;
        Ok(Response::new(decremented.into()))
    }

    #[tracing::instrument(ret)]
    async fn show(&self, _: Request<()>) -> Result<Response<i64>, Status> {
        let shown = self.bigint_show()?;
        Ok(Response::new(Self::cast_bigint(shown)?))
    }
    #[tracing::instrument(ret)]
    async fn bshow(&self, _: Request<()>) -> Result<Response<counter_pb::BigInt>, Status> {
        let shown = self.bigint_show()?;
        Ok(Response::new(shown.into()))
    }
    #[tracing::instrument(ret)]
    async fn reset(&self, _: Request<()>) -> Result<Response<i64>, Status> {
        let reset = self.bigint_reset()?;
        Ok(Response::new(Self::cast_bigint(reset)?))
    }
    #[tracing::instrument(ret)]
    async fn breset(&self, _: Request<()>) -> Result<Response<counter_pb::BigInt>, Status> {
        let reset = self.bigint_reset()?;
        Ok(Response::new(reset.into()))
    }
}

impl CounterImpl {
    pub fn new(initial_count: BigInt) -> Self {
        Self { counter: Arc::new(RwLock::new(initial_count)) }
    }

    pub fn bigint_increment(&self, value: BigInt) -> Result<BigInt, Status> {
        let mut counter = self.counter.write().map_err(|e| Status::internal(e.to_string()))?;
        *counter += value;
        Ok((*counter).clone())
    }
    pub fn bigint_show(&self) -> Result<BigInt, Status> {
        let counter = self.counter.read().map_err(|e| Status::internal(e.to_string()))?;
        Ok((*counter).clone())
    }
    pub fn bigint_reset(&self) -> Result<BigInt, Status> {
        let mut counter = self.counter.write().map_err(|e| Status::internal(e.to_string())).unwrap();
        *counter = BigInt::from(0);
        Ok((*counter).clone())
    }

    pub fn cast_bigint(value: BigInt) -> Result<i64, Status> {
        value.to_i64().ok_or_else(|| Status::invalid_argument(CounterError::TooLarge(value)))
    }
}

#[derive(Error, Debug)]
pub enum CounterError {
    #[error("{0} is too large to be converted as i64")]
    TooLarge(BigInt),
}
impl From<CounterError> for String {
    fn from(value: CounterError) -> Self {
        value.to_string()
    }
}
