use bytes::Bytes;
use http::HeaderMap;
use http_body::Body;

use crate::{
    error::Wrap,
    interface::{
        config::{HttpBody, HttpRequest},
        template::Template,
    },
    service::factory::RequestFactory,
};

impl<B> RequestFactory<http::Request<B>> for HttpRequest
where
    B: Body,
    HttpBody: BodyFactory<B>,
    Wrap: From<<HttpBody as BodyFactory<B>>::Error>,
{
    type Error = Wrap;
    fn produce(
        &self,
        destination: &http::Uri,
        target: &str,
        template: &Template,
    ) -> Result<http::Request<B>, Self::Error> {
        let HttpRequest { no_additional_headers, method, headers, body } = self;
        let uri = http::uri::Builder::from(destination.clone()).path_and_query(template.render(target)?).build()?;
        let unwrapped_method = method.as_ref().map(|m| (**m).clone()).unwrap_or_default();
        let unwrapped_headers: HeaderMap = headers.as_ref().map(|h| (**h).clone()).unwrap_or_default();
        // .into_iter().map(|(k, v)| (k, template.render_as_string(v))).collect(); // TODO template with header
        let (actual_body, additional_headers) = body.clone().unwrap_or_default().body_with_headers(template)?;

        let mut request = http::Request::builder().uri(uri).method(unwrapped_method).body(actual_body)?;
        let header_map = request.headers_mut();
        header_map.extend(unwrapped_headers);
        if !no_additional_headers {
            header_map.extend(additional_headers);
        }
        Ok(request)
    }
}

pub trait BodyFactory<B: Body> {
    type Error;
    fn produce(&self, template: &Template) -> Result<B, Self::Error>;
}
impl<B> BodyFactory<B> for HttpBody
where
    B: Body + From<Bytes> + Default,
{
    type Error = Wrap;
    fn produce(&self, template: &Template) -> Result<B, Self::Error> {
        match self {
            HttpBody::Empty => Ok(Default::default()),
            HttpBody::Plaintext(s) => Ok(Bytes::from(template.render(s).unwrap_or(s.to_string())).into()),
            #[cfg(feature = "json")]
            HttpBody::Json(_) => Ok(Bytes::from(serde_json::to_vec(&self)?).into()),
        }
    }
}
