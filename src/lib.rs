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
    pub async fn assault(self) -> error::RelentlessResult<Outcome> {
        let mut outcomes = Vec::new();
        // TODO async
        for config in self.configs {
            let (worker, cases) = config.instance()?;
            outcomes.push(worker.assault(cases).await?);
        }
        Ok(Outcome::new(outcomes))
    }
}

#[derive(Debug, Clone)]
pub struct Outcome {
    outcome: Vec<outcome::WorkerOutcome>,
}
impl Outcome {
    pub fn new(outcome: Vec<outcome::WorkerOutcome>) -> Self {
        Self { outcome }
    }
    pub fn success(&self) -> bool {
        self.outcome.iter().all(|o| o.success())
    }
    pub fn exit_code(&self) -> std::process::ExitCode {
        (!self.success() as u8).into()
    }
}
