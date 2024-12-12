use std::time::Duration;

use crate::interface::config::destinations::Destinations;

pub enum RequestResult<Res> {
    Response(Res),
    Timeout(Duration),
}

#[allow(async_fn_in_trait)] // TODO #[warn(async_fn_in_trait)] by default
pub trait Evaluator<Res> {
    type Message;
    async fn evaluate(&self, res: Destinations<RequestResult<Res>>, msg: &mut Vec<Self::Message>) -> bool;
}
