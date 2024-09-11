use std::str::FromStr;

use thiserror::Error;

use super::config::Testcase;

#[derive(Error, Debug)]
pub enum HttpError {
    #[error(transparent)]
    ReqwestError(#[from] reqwest::Error),

    #[error(transparent)]
    InvalidMethod(#[from] http::method::InvalidMethod),

    #[error(transparent)]
    InvalidUrl(#[from] url::ParseError),
}

impl Testcase {
    pub fn to_request(&self, host: &str) -> Result<reqwest::Request, HttpError> {
        // TODO post body
        let method = reqwest::Method::from_str(&self.method)?;
        let url = reqwest::Url::parse(host)?.join(&self.pathname)?;
        Ok(reqwest::Request::new(method, url))
    }
}
