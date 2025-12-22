use std::num::TryFromIntError;

use rand::distr::SampleString;
use rand_distr::Distribution;
use tonic::{Request, Response, Status};

pub mod pb {
    use std::convert::Infallible;

    tonic::include_proto!("random");

    impl super::DistributionParameter<rand_distr::StandardUniform> for Standard {
        type Error = Infallible;
        fn distribution(&self) -> Result<rand_distr::StandardUniform, Self::Error> {
            Ok(rand_distr::StandardUniform)
        }
    }
    impl super::DistributionParameter<rand_distr::StandardNormal> for Normal {
        type Error = Infallible;
        fn distribution(&self) -> Result<rand_distr::StandardNormal, Self::Error> {
            Ok(rand_distr::StandardNormal)
        }
    }
    impl super::DistributionParameter<rand_distr::Binomial> for Binomial {
        type Error = rand_distr::BinomialError;
        fn distribution(&self) -> Result<rand_distr::Binomial, Self::Error> {
            let &Self { n, p } = self;
            rand_distr::Binomial::new(n, p)
        }
    }
    impl super::DistributionParameter<rand_distr::Uniform<i64>> for UniformInt {
        type Error = rand_distr::uniform::Error;
        fn distribution(&self) -> Result<rand_distr::Uniform<i64>, Self::Error> {
            let &Self { min, max, inclusive } = self;
            if inclusive {
                rand_distr::Uniform::new_inclusive(min, max)
            } else {
                rand_distr::Uniform::new(min, max)
            }
        }
    }
    impl super::DistributionParameter<rand_distr::Uniform<f64>> for UniformFloat {
        type Error = rand_distr::uniform::Error;
        fn distribution(&self) -> Result<rand_distr::Uniform<f64>, Self::Error> {
            let &Self { min, max, inclusive } = self;
            if inclusive {
                rand_distr::Uniform::new_inclusive(min, max)
            } else {
                rand_distr::Uniform::new(min, max)
            }
        }
    }
    impl super::DistributionParameter<rand_distr::Alphanumeric> for Alphanumeric {
        type Error = Infallible;
        fn distribution(&self) -> Result<rand_distr::Alphanumeric, Self::Error> {
            Ok(rand_distr::Alphanumeric)
        }
    }
}

pub trait DistributionParameter<D> {
    type Error;
    fn distribution(&self) -> Result<D, Self::Error>;
    fn random_int(&self) -> Result<i64, Self::Error>
    where
        D: Distribution<i64>,
    {
        self.distribution().map(|d| d.sample(&mut rand::rng()))
    }
    fn random_uint(&self) -> Result<u64, Self::Error>
    where
        D: Distribution<u64>,
    {
        self.distribution().map(|d| d.sample(&mut rand::rng()))
    }
    fn random_float(&self) -> Result<f64, Self::Error>
    where
        D: Distribution<f64>,
    {
        self.distribution().map(|d| d.sample(&mut rand::rng()))
    }
    fn random_string(&self, len: usize) -> Result<String, Self::Error>
    where
        D: SampleString,
    {
        self.distribution().map(|d| d.sample_string(&mut rand::rng(), len))
    }
}

#[derive(Debug, Default)]
pub struct RandomImpl;

#[tonic::async_trait]
impl pb::random_server::Random for RandomImpl {
    async fn int(&self, request: Request<pb::DistributionInt>) -> Result<Response<pb::RandomInt>, Status> {
        let pb::DistributionInt { distribution } = request.into_inner();
        let value = match distribution.unwrap() {
            pb::distribution_int::Distribution::Standard(s) => s.random_int().unwrap_or_else(|_| unreachable!()),
            pb::distribution_int::Distribution::Binomial(b) => b
                .random_uint()
                .map_err(|e| Status::invalid_argument(e.to_string()))?
                .try_into()
                .map_err(|e: TryFromIntError| Status::out_of_range(e.to_string()))?,
            pb::distribution_int::Distribution::Uniform(u) => {
                u.random_int().map_err(|e| Status::invalid_argument(e.to_string()))?
            }
        };
        Ok(Response::new(pb::RandomInt { value }))
    }
    async fn float(&self, request: Request<pb::DistributionFloat>) -> Result<Response<pb::RandomFloat>, Status> {
        let pb::DistributionFloat { distribution } = request.into_inner();
        let value = match distribution.unwrap() {
            pb::distribution_float::Distribution::Standard(s) => s.random_float().unwrap_or_else(|_| unreachable!()),
            pb::distribution_float::Distribution::Normal(n) => n.random_float().unwrap_or_else(|_| unreachable!()),
            pb::distribution_float::Distribution::Uniform(u) => {
                u.random_float().map_err(|e| Status::invalid_argument(e.to_string()))?
            }
        };
        Ok(Response::new(pb::RandomFloat { value }))
    }
    async fn string(&self, request: Request<pb::DistributionString>) -> Result<Response<pb::RandomString>, Status> {
        let pb::DistributionString { length, distribution } = request.into_inner();
        let value = match distribution.unwrap() {
            pb::distribution_string::Distribution::Standard(s) => {
                s.random_string(length as usize).unwrap_or_else(|_| unreachable!())
            }
            pb::distribution_string::Distribution::Alphanumeric(a) => {
                a.random_string(length as usize).unwrap_or_else(|_| unreachable!())
            }
        };
        Ok(Response::new(pb::RandomString { value }))
    }
}
