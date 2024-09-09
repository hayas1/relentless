use std::str::FromStr;

use thiserror::Error;

use super::Http;

#[derive(Error, Debug)]
pub enum HttpError {
    #[error(transparent)]
    ReqwestError(#[from] reqwest::Error),

    #[error(transparent)]
    InvalidMethod(#[from] http::method::InvalidMethod),

    #[error(transparent)]
    InvalidUrl(#[from] url::ParseError),
}

impl Http {
    pub fn to_request(&self, host: &str) -> Result<reqwest::Request, HttpError> {
        let method = reqwest::Method::from_str(&self.method)?;
        let url = reqwest::Url::parse(host)
            .unwrap()
            .join(&self.pathname)
            .unwrap();
        Ok(reqwest::Request::new(method, url))
    }
}
