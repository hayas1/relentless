use std::{
    fmt::{Display, Formatter, Write},
    process::ExitCode,
};

use http_body::Body;
use http_body_util::BodyExt;

use crate::{
    command::Assault,
    config::{Testcase, WorkerConfig},
    error::{HttpError, RelentlessError, RelentlessResult},
};

#[allow(async_fn_in_trait)] // TODO #[warn(async_fn_in_trait)] by default
pub trait Evaluator<Res> {
    type Error;
    async fn evaluate<I: IntoIterator<Item = Res>>(iter: I) -> Result<bool, Self::Error>;
}
pub struct Compare {} // TODO enum ?
impl<ResB: Body> Evaluator<http::Response<ResB>> for Compare {
    type Error = RelentlessError;
    async fn evaluate<I: IntoIterator<Item = http::Response<ResB>>>(iter: I) -> Result<bool, Self::Error> {
        let mut v = Vec::new();
        for res in iter {
            let status = res.status();
            let body =
                BodyExt::collect(res).await.map(|buf| buf.to_bytes()).map_err(|_| HttpError::CannotConvertBody)?;
            v.push((status, body));
        }
        let pass = v.windows(2).all(|w| w[0] == w[1]);
        Ok(pass)
    }
}

pub struct Status {} // TODO enum ?
impl<ResB> Evaluator<http::Response<ResB>> for Status {
    type Error = RelentlessError;
    async fn evaluate<I: IntoIterator<Item = http::Response<ResB>>>(iter: I) -> Result<bool, Self::Error> {
        let pass = iter.into_iter().all(|res| res.status().is_success());
        Ok(pass)
    }
}

/// TODO document
#[derive(Debug, Clone)]
pub struct Outcome {
    outcome: Vec<WorkerOutcome>,
}
impl Outcome {
    pub fn new(outcome: Vec<WorkerOutcome>) -> Self {
        Self { outcome }
    }
    pub fn pass(&self) -> bool {
        self.outcome.iter().all(|o| o.pass())
    }
    pub fn allow(&self, strict: bool) -> bool {
        self.outcome.iter().all(|o| o.allow(strict))
    }
    pub fn exit_code(&self, strict: bool) -> ExitCode {
        (!self.allow(strict) as u8).into()
    }
    // TODO trait ?
    pub fn write(&self, w: &mut OutcomeWriter, cmd: &Assault) -> std::fmt::Result {
        let side = console::Emoji("🔥", "");
        writeln!(w, "{} Relentless Assault Result {}", side, side)?;
        w.increment();
        for outcome in &self.outcome {
            outcome.write(w, cmd)?;
        }
        w.decrement();
        Ok(())
    }
}

/// TODO document
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
    pub fn write(&self, w: &mut OutcomeWriter, cmd: &Assault) -> std::fmt::Result {
        let side = console::Emoji("📂", "");
        writeln!(w, "{} {}", side, self.config.name.as_ref().unwrap_or(&"testcases".to_string()))?;
        w.increment();
        for outcome in &self.outcome {
            outcome.write(w, cmd)?;
        }
        w.decrement();
        Ok(())
    }
}

/// TODO document
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
    pub fn write(&self, w: &mut OutcomeWriter, cmd: &Assault) -> std::fmt::Result {
        let side = if self.pass() { console::Emoji("✅", "") } else { console::Emoji("❌", "") };
        writeln!(w, "{} {}", side, self.testcase.target)?;
        if let Some(desc) = &self.testcase.description {
            w.increment();
            writeln!(w, "  {} {}", console::Emoji("📝", ""), desc)?;
            w.decrement();
        }
        if !self.pass() && self.allow(cmd.strict) {
            w.increment();
            writeln!(w, "  {} {}", console::Emoji("👟", ""), console::style("this testcase is allowed").green())?;
            w.decrement();
        }
        Ok(())
    }
}

pub struct OutcomeWriter {
    pub indent: usize,
    pub buf: String, // TODO String?
}
impl OutcomeWriter {
    pub fn new(indent: usize) -> Self {
        let buf = String::new();
        Self { indent, buf }
    }
    pub fn increment(&mut self) {
        self.indent += 1;
    }
    pub fn decrement(&mut self) {
        self.indent -= 1;
    }
}
impl Write for OutcomeWriter {
    fn write_str(&mut self, s: &str) -> std::fmt::Result {
        // TODO better implementation?
        if s.contains("\n") {
            for line in s.lines() {
                writeln!(self.buf, "{}{}", "  ".repeat(self.indent), line)?;
            }
        } else {
            write!(self.buf, "{}", s)?;
        }
        Ok(())
    }
}
impl Display for OutcomeWriter {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.buf)
    }
}
