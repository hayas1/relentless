use reqwest::Response;

use crate::error::RelentlessResult;

#[allow(async_fn_in_trait)] // TODO #[warn(async_fn_in_trait)] by default
pub trait Evaluator<Res> {
    // TODO remove description from interface
    async fn evaluate<I: IntoIterator<Item = Res>>(
        description: Option<String>,
        iter: I,
    ) -> RelentlessResult<CaseOutcome>;
}
pub struct Compare {} // TODO enum ?
impl Evaluator<Response> for Compare {
    async fn evaluate<I: IntoIterator<Item = Response>>(
        description: Option<String>,
        iter: I,
    ) -> RelentlessResult<CaseOutcome> {
        let mut v = Vec::new();
        for res in iter {
            v.push((res.status(), res.text().await?));
        }
        let success = v.windows(2).all(|w| w[0] == w[1]);
        Ok(CaseOutcome::new(description, success))
    }
}

pub struct Status {} // TODO enum ?
impl Evaluator<Response> for Status {
    async fn evaluate<I: IntoIterator<Item = Response>>(
        description: Option<String>,
        iter: I,
    ) -> RelentlessResult<CaseOutcome> {
        let success = iter.into_iter().all(|res| res.status().is_success());
        Ok(CaseOutcome::new(description, success))
    }
}

#[derive(Debug, Clone)]
pub struct CaseOutcome {
    description: Option<String>,
    success: bool,
}
impl CaseOutcome {
    pub fn new(description: Option<String>, success: bool) -> Self {
        Self { description, success }
    }
    pub fn success(&self) -> bool {
        self.success
    }
}

#[derive(Debug, Clone)]
pub struct WorkerOutcome {
    name: Option<String>,
    outcome: Vec<CaseOutcome>,
}
impl WorkerOutcome {
    pub fn new(name: Option<String>, outcome: Vec<CaseOutcome>) -> Self {
        Self { name, outcome }
    }
    pub fn success(&self) -> bool {
        self.outcome.iter().all(|o| o.success())
    }
}
