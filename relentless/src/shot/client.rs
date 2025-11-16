use crate::{evaluator::Evaluator, generator::Generator, shot::testcase::Profile};

#[trait_variant::make(Send)]
pub trait Client<S>: Sized {
    type Generator: Generator<S>;
    type Evaluator: Evaluator<S>;
    type Error;
    async fn connect(
        destination: &http::Uri,
        profile: &Profile<Self::Generator, Self::Evaluator>,
    ) -> Result<Self, Self::Error>;
}
