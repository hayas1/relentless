use std::collections::HashMap;

use bytes::Bytes;
use http::{
    header::{CONTENT_LENGTH, CONTENT_TYPE},
    HeaderMap,
};
use http_body::Body;
use mime::{Mime, APPLICATION_JSON, TEXT_PLAIN};
use serde::{Deserialize, Serialize};

use crate::{
    assault::factory::RequestFactory,
    error::{Wrap, WrappedResult},
    interface::{
        helper::{coalesce::Coalesce, http_serde_priv, is_default::IsDefault},
        template::Template,
    },
};

#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "kebab-case")]
pub struct HttpRequest {
    #[serde(default, skip_serializing_if = "IsDefault::is_default")]
    pub no_additional_headers: bool,
    #[serde(default, skip_serializing_if = "IsDefault::is_default")]
    pub method: Option<http_serde_priv::Method>,
    #[serde(default, skip_serializing_if = "IsDefault::is_default")]
    pub headers: Option<http_serde_priv::HeaderMap>,
    #[serde(default, skip_serializing_if = "IsDefault::is_default")]
    pub body: HttpBody,
}
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "kebab-case", untagged)]
pub enum HttpBody {
    #[default]
    Empty,
    Plaintext(String),
    #[cfg(feature = "json")]
    Json(HashMap<String, String>),
}
impl HttpBody {
    pub fn body_with_headers<ReqB>(&self, template: &Template) -> WrappedResult<(ReqB, HeaderMap)>
    where
        ReqB: Body,
        Self: BodyFactory<ReqB>,
        Wrap: From<<Self as BodyFactory<ReqB>>::Error>,
    {
        let mut headers = HeaderMap::new();
        self.content_type()
            .map(|t| headers.insert(CONTENT_TYPE, t.as_ref().parse().unwrap_or_else(|_| unreachable!())));
        let body = self.produce(template)?;
        body.size_hint().exact().filter(|size| *size > 0).map(|size| headers.insert(CONTENT_LENGTH, size.into())); // TODO remove ?
        Ok((body, headers))
    }
    pub fn content_type(&self) -> Option<Mime> {
        match self {
            HttpBody::Empty => None,
            HttpBody::Plaintext(_) => Some(TEXT_PLAIN),
            #[cfg(feature = "json")]
            HttpBody::Json(_) => Some(APPLICATION_JSON),
        }
    }
}

impl Coalesce for HttpRequest {
    fn coalesce(self, other: &Self) -> Self {
        Self {
            no_additional_headers: self.no_additional_headers || other.no_additional_headers,
            method: self.method.or(other.method.clone()),
            headers: self.headers.or(other.headers.clone()),
            body: self.body.coalesce(&other.body),
        }
    }
}
impl Coalesce for HttpBody {
    fn coalesce(self, other: &Self) -> Self {
        match self {
            HttpBody::Empty => other.clone(),
            _ => self,
        }
    }
}

impl<B> RequestFactory<http::Request<B>> for HttpRequest
where
    B: Body,
    HttpBody: BodyFactory<B>,
    Wrap: From<<HttpBody as BodyFactory<B>>::Error>,
{
    type Error = crate::Error;
    fn produce(
        &self,
        destination: &http::Uri,
        target: &str,
        template: &Template,
    ) -> Result<http::Request<B>, Self::Error> {
        let HttpRequest { no_additional_headers, method, headers, body } = self;
        let uri = http::uri::Builder::from(destination.clone())
            .path_and_query(template.render(target)?)
            .build()
            .map_err(Wrap::error)?;
        let unwrapped_method = method.as_ref().map(|m| (**m).clone()).unwrap_or_default();
        let unwrapped_headers: HeaderMap = headers.as_ref().map(|h| (**h).clone()).unwrap_or_default();
        // .into_iter().map(|(k, v)| (k, template.render_as_string(v))).collect(); // TODO template with header
        let (actual_body, additional_headers) = body.clone().body_with_headers(template)?;

        let mut request =
            http::Request::builder().uri(uri).method(unwrapped_method).body(actual_body).map_err(Wrap::error)?;
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
    type Error = crate::Error;
    fn produce(&self, template: &Template) -> Result<B, Self::Error> {
        match self {
            HttpBody::Empty => Ok(Default::default()),
            HttpBody::Plaintext(s) => Ok(Bytes::from(template.render(s).unwrap_or(s.to_string())).into()),
            #[cfg(feature = "json")]
            HttpBody::Json(_) => Ok(Bytes::from(serde_json::to_vec(&self).map_err(Wrap::error)?).into()),
        }
    }
}
