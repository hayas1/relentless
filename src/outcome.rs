use reqwest::Response;

use crate::{
    config::{Testcase, WorkerConfig},
    error::RelentlessResult,
};

#[allow(async_fn_in_trait)] // TODO #[warn(async_fn_in_trait)] by default
pub trait Evaluator<Res> {
    async fn evaluate<I: IntoIterator<Item = Res>>(iter: I) -> RelentlessResult<bool>;
}
pub struct Compare {} // TODO enum ?
impl Evaluator<Response> for Compare {
    async fn evaluate<I: IntoIterator<Item = Response>>(iter: I) -> RelentlessResult<bool> {
        let mut v = Vec::new();
        for res in iter {
            v.push((res.status(), res.text().await?));
        }
        let pass = v.windows(2).all(|w| w[0] == w[1]);
        Ok(pass)
    }
}

pub struct Status {} // TODO enum ?
impl Evaluator<Response> for Status {
    async fn evaluate<I: IntoIterator<Item = Response>>(iter: I) -> RelentlessResult<bool> {
        let pass = iter.into_iter().all(|res| res.status().is_success());
        Ok(pass)
    }
}

#[derive(Debug, Clone)]
pub struct CaseOutcome {
    testcase: Testcase,
    pass: bool,
}
impl CaseOutcome {
    pub fn new(testcase: Testcase, pass: bool) -> Self {
        Self { testcase, pass }
    }
    pub fn pass(&self) -> bool {
        self.pass
    }
    pub fn allow(&self, strict: bool) -> bool {
        let allowed = self.testcase.attr.allow;
        self.pass() || !strict && allowed
    }
}

#[derive(Debug, Clone)]
pub struct WorkerOutcome {
    config: WorkerConfig,
    outcome: Vec<CaseOutcome>,
}
impl WorkerOutcome {
    pub fn new(config: WorkerConfig, outcome: Vec<CaseOutcome>) -> Self {
        Self { config, outcome }
    }
    pub fn pass(&self) -> bool {
        self.outcome.iter().all(|o| o.pass())
    }
    pub fn allow(&self, strict: bool) -> bool {
        self.outcome.iter().all(|o| o.allow(strict))
    }
}
