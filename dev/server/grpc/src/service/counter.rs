use std::sync::{Arc, RwLock};

use num::{BigInt, ToPrimitive};
use pb::{counter_server::Counter, CounterReply, CounterRequest};
use tonic::{Request, Response, Status};

pub mod pb {
    tonic::include_proto!("counter");

    impl From<num::bigint::Sign> for Sign {
        fn from(sign: num::bigint::Sign) -> Self {
            match sign {
                num::bigint::Sign::NoSign => Sign::NoSign,
                num::bigint::Sign::Plus => Sign::Plus,
                num::bigint::Sign::Minus => Sign::Minus,
            }
        }
    }
    impl From<Sign> for num::bigint::Sign {
        fn from(sign: Sign) -> Self {
            match sign {
                Sign::NoSign => num::bigint::Sign::NoSign,
                Sign::Plus => num::bigint::Sign::Plus,
                Sign::Minus => num::bigint::Sign::Minus,
            }
        }
    }
    impl From<num::BigInt> for BigInt {
        fn from(value: num::BigInt) -> Self {
            let (sign, repr) = value.to_bytes_be();
            BigInt { sign: Sign::from(sign).into(), repr }
        }
    }
    impl From<BigInt> for num::BigInt {
        fn from(value: BigInt) -> Self {
            let (sign, repr) = (value.sign(), value.repr);
            num::BigInt::from_bytes_be(sign.into(), &repr)
        }
    }
}

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
    async fn bincrement(&self, request: Request<pb::BigInt>) -> Result<Response<pb::BigInt>, Status> {
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
