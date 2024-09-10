use crate::error::RelentlessResult;
use format::{Format, Protocol, Testcases};
use reqwest::Request;
use std::path::Path;
use tower::Service;

pub mod format;
pub mod http;

impl Testcases {
    pub fn import<P: AsRef<Path>>(path: P) -> RelentlessResult<Self> {
        Ok(Format::from_path(path.as_ref())?.import_testcase(path.as_ref())?)
    }

    pub async fn run(&self) -> RelentlessResult<()> {
        match self.protocol {
            Protocol::Http(ref http) => {
                let requests = http
                    .iter()
                    .map(|h| self.host.iter().map(|(_, host)| h.to_request(host)))
                    .flatten(); // TODO do not flatten (for compare test)
                for r in requests {
                    let client = reqwest::Client::new();
                    self.request(client, r?).await?;
                }
            }
            _ => unimplemented!(),
        }
        Ok(())
    }

    pub async fn request<S: Service<Request>>(
        &self,
        mut service: S,
        request: Request,
    ) -> Result<S::Response, S::Error> {
        service.call(request).await
    }
}
