use tower::{Layer, Service};

#[trait_variant::make(Send)]
pub trait Contract<S, Q>: Sized + Layer<S>
where
    Self::Service: Service<Self::Request>,
{
    type Request;
    type Error;

    async fn new(service: S, request: Q) -> Result<Self, Self::Error>;
}
