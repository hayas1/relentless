use crate::interface::template::Template;

pub trait RequestFactory<R> {
    type Error;
    fn produce(&self, destination: &http::Uri, target: &str, template: &Template) -> Result<R, Self::Error>;
}
