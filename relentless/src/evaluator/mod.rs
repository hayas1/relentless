use crate::shot::destinations::Destinations;

pub mod expect;
pub mod json;
pub mod plaintext;

#[trait_variant::make(Send)]
pub trait Evaluator<S> {
    type Response;
    async fn evaluate(&self, res: Destinations<Self::Response>) -> bool;
}
