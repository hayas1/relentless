use std::sync::{Arc, RwLock};

use num::{BigInt, ToPrimitive};
use thiserror::Error;
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

#[derive(Debug, Default)]
pub struct CounterImpl {
    pub counter: Arc<RwLock<BigInt>>,
}

#[tonic::async_trait]
impl pb::counter_server::Counter for CounterImpl {
    #[tracing::instrument(ret)]
    async fn increment(&self, request: Request<i64>) -> Result<Response<i64>, Status> {
        let value = BigInt::from(request.into_inner());
        let incremented = self.bigint_increment(value)?;
        Ok(Response::new(Self::cast_bigint(incremented)?))
    }
    #[tracing::instrument(ret)]
    async fn bincrement(&self, request: Request<pb::BigInt>) -> Result<Response<pb::BigInt>, Status> {
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
    async fn bdecrement(&self, request: Request<pb::BigInt>) -> Result<Response<pb::BigInt>, Status> {
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
    async fn bshow(&self, _: Request<()>) -> Result<Response<pb::BigInt>, Status> {
        let shown = self.bigint_show()?;
        Ok(Response::new(shown.into()))
    }
    #[tracing::instrument(ret)]
    async fn reset(&self, _: Request<()>) -> Result<Response<i64>, Status> {
        let reset = self.bigint_reset()?;
        Ok(Response::new(Self::cast_bigint(reset)?))
    }
    #[tracing::instrument(ret)]
    async fn breset(&self, _: Request<()>) -> Result<Response<pb::BigInt>, Status> {
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

#[cfg(test)]
mod tests {
    use pb::{counter_client::CounterClient, counter_server::CounterServer};
    use tonic::Code;

    use super::*;

    #[tokio::test]
    async fn test_counter_basic() {
        let server = CounterServer::new(CounterImpl::new(BigInt::from(0)));
        let mut client = CounterClient::new(server);

        assert_eq!(client.increment(1).await.unwrap().into_inner(), 1);
        assert_eq!(client.increment(2).await.unwrap().into_inner(), 3);

        assert_eq!(client.decrement(1).await.unwrap().into_inner(), 2);
        assert_eq!(client.decrement(3).await.unwrap().into_inner(), -1);

        assert_eq!(client.show(()).await.unwrap().into_inner(), -1);
        assert_eq!(client.reset(()).await.unwrap().into_inner(), 0);
    }

    #[tokio::test]
    async fn test_counter_too_large_bigint() {
        let server = CounterServer::new(CounterImpl::new(BigInt::from(0)));
        let mut client = CounterClient::new(server);

        let large: BigInt = "9999999999999999999999999999999".parse().unwrap();
        assert_eq!(
            client.bincrement(pb::BigInt::from(large.clone())).await.unwrap().into_inner(),
            large.clone().into()
        );

        let err = client.show(()).await.unwrap_err();
        assert_eq!(err.code(), Code::InvalidArgument);
        assert_eq!(err.message(), CounterError::TooLarge(large.clone()).to_string());

        assert_eq!(client.bshow(()).await.unwrap().into_inner(), large.clone().into());

        assert_eq!(client.breset(()).await.unwrap().into_inner(), BigInt::from(0).into());
        assert_eq!(client.show(()).await.unwrap().into_inner(), 0);
    }
}
