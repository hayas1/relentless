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
        let pass = v.windows(2).all(|w| w[0] == w[1]);
        Ok(CaseOutcome::new(description, pass))
    }
}

pub struct Status {} // TODO enum ?
impl Evaluator<Response> for Status {
    async fn evaluate<I: IntoIterator<Item = Response>>(
        description: Option<String>,
        iter: I,
    ) -> RelentlessResult<CaseOutcome> {
        let pass = iter.into_iter().all(|res| res.status().is_success());
        Ok(CaseOutcome::new(description, pass))
    }
}

#[derive(Debug, Clone)]
pub struct CaseOutcome {
    description: Option<String>,
    pass: bool,
}
impl CaseOutcome {
    pub fn new(description: Option<String>, pass: bool) -> Self {
        Self { description, pass }
    }
    pub fn pass(&self) -> bool {
        self.pass
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
    pub fn pass(&self) -> bool {
        self.outcome.iter().all(|o| o.pass())
    }
}
