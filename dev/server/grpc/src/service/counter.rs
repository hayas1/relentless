use std::sync::{Arc, RwLock};

use num::BigInt;
use pb::{counter_server::Counter, CounterReply, CounterRequest};
use tonic::{Request, Response, Status};

pub mod pb {
    tonic::include_proto!("counter");
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Default, Hash)]
pub struct CounterState {
    // pub count: BigInt,
    pub count: i64,
}

#[derive(Debug, Default)]
pub struct CounterImpl {
    // pub counter: Arc<RwLock<BigInt>>,
    pub counter: Arc<RwLock<i64>>,
}

#[tonic::async_trait]
impl Counter for CounterImpl {
    #[tracing::instrument(ret)]
    async fn incr(&self, request: Request<i64>) -> Result<Response<i64>, Status> {
        let value = request.into_inner();
        let mut counter = self.counter.write().map_err(|e| Status::internal(e.to_string()))?;
        *counter += value;
        Ok(Response::new(*counter))
    }
    #[tracing::instrument(ret)]
    async fn increment(&self, request: Request<CounterRequest>) -> Result<Response<CounterReply>, Status> {
        let value = request.into_inner().value;
        let mut counter = self.counter.write().map_err(|e| Status::internal(e.to_string()))?;
        *counter += value;
        Ok(Response::new(CounterReply { value: *counter }))
    }
}
