use reqwest::Response;

use crate::error::RelentlessResult;

#[allow(async_fn_in_trait)] // TODO #[warn(async_fn_in_trait)] by default
pub trait Evaluator<Res> {
    // TODO remove description from interface
    async fn evaluate<I: IntoIterator<Item = Res>>(description: Option<String>, iter: I) -> RelentlessResult<Outcome>;
}
pub struct Compare {}
impl Evaluator<Response> for Compare {
    async fn evaluate<I: IntoIterator<Item = Response>>(
        description: Option<String>,
        iter: I,
    ) -> RelentlessResult<Outcome> {
        let mut v = Vec::new();
        for res in iter {
            v.push((res.status(), res.text().await?));
        }
        let status = v.windows(2).all(|w| w[0] == w[1]);
        Ok(Outcome::new(description, status))
    }
}

#[derive(Debug, Clone)]
pub struct Outcome {
    description: Option<String>,
    status: bool,
}
impl Outcome {
    pub fn new(description: Option<String>, status: bool) -> Self {
        Self { description, status }
    }
}
