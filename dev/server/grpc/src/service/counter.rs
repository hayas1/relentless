use std::sync::{Arc, RwLock};

use num::{BigInt, ToPrimitive};
use pb::{counter_server::Counter, BInt, CounterReply, CounterRequest};
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
    impl From<num::BigInt> for BInt {
        fn from(value: num::BigInt) -> Self {
            let (sign, value) = value.to_bytes_be();
            BInt { sign: Sign::from(sign).into(), value }
        }
    }
    impl From<BInt> for num::BigInt {
        fn from(value: BInt) -> Self {
            let (sign, value) = (value.sign(), value.value);
            num::BigInt::from_bytes_be(sign.into(), &value)
        }
    }
}
// impl From<Sign> for pb::Sign {
//     fn from(sign: Sign) -> Self {
//         match sign {
//             Sign::NoSign => pb::Sign::NoSign,
//             Sign::Plus => pb::Sign::Plus,
//             Sign::Minus => pb::Sign::Minus,
//         }
//     }
// }
// impl From<pb::Sign> for Sign {
//     fn from(sign: pb::Sign) -> Self {
//         match sign {
//             pb::Sign::NoSign => Sign::NoSign,
//             pb::Sign::Plus => Sign::Plus,
//             pb::Sign::Minus => Sign::Minus,
//         }
//     }
// }
// impl From<BigInt> for pb::BInt {
//     fn from(value: BigInt) -> Self {
//         let (sign, value) = value.to_bytes_be();
//         BInt { sign: pb::Sign::from(sign).into(), value }
//     }
// }
// impl From<pb::BInt> for BigInt {
//     fn from(value: pb::BInt) -> Self {
//         let pb::BInt { sign, value } = value;
//         BigInt::from_bytes_be(pb::Sign::try_from(sign).unwrap().into(), &value)
//     }
// }

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
    async fn bincrement(&self, request: Request<BInt>) -> Result<Response<BInt>, Status> {
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
