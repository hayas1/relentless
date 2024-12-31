use std::sync::{Arc, RwLock};

use num::{BigInt, ToPrimitive};
use relentless_dev_server_grpc_entity::counter_pb::{self, counter_server::Counter, CounterReply, CounterRequest};
use tonic::{Request, Response, Status};

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Default, Hash)]
pub struct CounterState {
    pub count: BigInt,
}

#[derive(Debug, Default)]
pub struct CounterImpl {
    pub counter: Arc<RwLock<BigInt>>,
}

#[tonic::async_trait]
impl Counter for CounterImpl {
    #[tracing::instrument(ret)]
    async fn incr(&self, request: Request<i64>) -> Result<Response<i64>, Status> {
        let value = request.into_inner();
        let incremented = self.bigint_increment(BigInt::from(value))?;
        Ok(Response::new(incremented.to_i64().unwrap()))
    }
    #[tracing::instrument(ret)]
    async fn increment(&self, request: Request<CounterRequest>) -> Result<Response<CounterReply>, Status> {
        let CounterRequest { value } = request.into_inner();
        let incremented = self.bigint_increment(BigInt::from(value))?;
        Ok(Response::new(CounterReply { value: incremented.to_i64().unwrap() }))
    }
    #[tracing::instrument(ret)]
    async fn bincrement(&self, request: Request<counter_pb::BigInt>) -> Result<Response<counter_pb::BigInt>, Status> {
        let bint = request.into_inner();
        let incremented = self.bigint_increment(bint.into())?;
        Ok(Response::new(incremented.into()))
    }
}

impl CounterImpl {
    pub fn bigint_increment(&self, value: BigInt) -> Result<BigInt, Status> {
        let mut counter = self.counter.write().map_err(|e| Status::internal(e.to_string()))?;
        *counter += value;
        Ok((*counter).clone())
    }
}
