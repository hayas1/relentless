pub mod config;
pub mod error;
pub mod outcome;
pub mod worker;

#[derive(Debug, Clone)]
pub struct Relentless {
    configs: Vec<config::Config>,
}
impl Relentless {
    /// TODO document
    pub fn new(configs: Vec<config::Config>) -> Self {
        Self { configs }
    }
    /// TODO document
    pub fn read_paths<I: IntoIterator<Item = P>, P: AsRef<std::path::Path>>(paths: I) -> error::RelentlessResult<Self> {
        let configs = paths.into_iter().map(config::Config::read).collect::<error::RelentlessResult<Vec<_>>>()?;
        Ok(Self::new(configs))
    }
    /// TODO document
    pub fn read_dir<P: AsRef<std::path::Path>>(path: P) -> error::RelentlessResult<Self> {
        let configs = config::Config::read_dir(path)?;
        Ok(Self::new(configs))
    }

    /// TODO document
    pub async fn assault(self) -> error::RelentlessResult<RelentlessOutcome> {
        let mut outcomes = Vec::new();
        // TODO async
        for config in self.configs {
            let (worker, cases) = config.instance()?;
            outcomes.push(worker.assault(cases).await?);
        }
        Ok(RelentlessOutcome::new(outcomes))
    }
}

#[derive(Debug, Clone)]
pub struct RelentlessOutcome {
    outcome: Vec<Vec<outcome::Outcome>>,
}
impl RelentlessOutcome {
    pub fn new(outcome: Vec<Vec<outcome::Outcome>>) -> Self {
        Self { outcome }
    }
    pub fn exit_code(&self) -> std::process::ExitCode {
        let success = self.outcome.iter().all(|v| v.iter().all(|o| o.status()));
        (!success as u8).into()
    }
}
