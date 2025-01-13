use bytes::Bytes;
use http_body::Body;
use relentless::interface::command::{Assault, Relentless};
use serde::{Deserialize, Serialize};

use crate::{evaluate::HttpResponse, factory::HttpRequest};

#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct HttpAssault {
    pub relentless: Relentless,
}
impl<ReqB, ResB> Assault<http::Request<ReqB>, http::Response<ResB>> for HttpAssault
where
    ReqB: Body + From<Bytes> + Default,
    ResB: Body + From<Bytes> + Default,
    ResB::Error: std::error::Error + Send + Sync + 'static,
{
    type Request = HttpRequest;
    type Response = HttpResponse;
    fn command(&self) -> &Relentless {
        &self.relentless
    }
}
