#[trait_variant::make(Send)]
pub trait Generator<S>: Sized {
    type Request;
    type Error;

    async fn generate(
        &self,
        service: S,
        destination: &http::Uri,
        target: &str,
        // template: &Template,
    ) -> Result<Self::Request, Self::Error>;
}
