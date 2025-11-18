#[trait_variant::make(Send)]
pub trait Generator: Sized {
    type Request;
    type Error;

    async fn generate(
        &self,
        destination: &http::Uri,
        target: &str,
        // template: &Template,
    ) -> Result<Self::Request, Self::Error>;
}
