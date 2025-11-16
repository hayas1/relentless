#[trait_variant::make(Send)]
pub trait Generator<S>: Sized {
    type Output;
    type Error;

    async fn generate(
        &self,
        service: S,
        destination: &http::Uri,
        target: &str,
        // template: &Template,
    ) -> Result<Self::Output, Self::Error>;
}
