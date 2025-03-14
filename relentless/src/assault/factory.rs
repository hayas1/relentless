use crate::interface::template::Template;

#[allow(async_fn_in_trait)] // TODO #[warn(async_fn_in_trait)] by default
pub trait RequestFactory<R, S> {
    type Error;
    async fn produce(
        &self,
        service: S,
        destination: &http::Uri,
        target: &str,
        template: &Template,
    ) -> Result<R, Self::Error>;
}
