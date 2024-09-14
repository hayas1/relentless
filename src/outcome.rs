use reqwest::Response;

use crate::{config::Attribute, error::RelentlessResult};

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
    description: Option<String>,
    pass: bool,
    attr: Attribute,
}
impl CaseOutcome {
    pub fn new(description: Option<String>, pass: bool, attr: Attribute) -> Self {
        Self { description, pass, attr }
    }
    pub fn pass(&self) -> bool {
        // TODO option: do not pass, but status code will be success
        let strict = false;
        // if strict {
        //     self.pass
        // } else {
        //     self.pass || self.attr.invalid
        // }
        self.pass || !strict && self.attr.invalid
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
