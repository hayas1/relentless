use std::{
    fmt::{Display, Formatter, Write as _},
    process::ExitCode,
};

use http_body::Body;
use http_body_util::BodyExt;

use crate::{
    command::Assault,
    config::{Testcase, WorkerConfig},
    error::{HttpError, RelentlessError},
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
    pub fn write<T: std::io::Write>(&self, w: &mut OutcomeWriter<T>, cmd: &Assault) -> std::fmt::Result {
        for outcome in &self.outcome {
            outcome.write(w, cmd)?;
        }
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
    pub fn write<T: std::io::Write>(&self, w: &mut OutcomeWriter<T>, cmd: &Assault) -> std::fmt::Result {
        let side = console::Emoji("🚀", "");
        writeln!(w, "{} {}", side, self.config.name.as_ref().unwrap_or(&"testcases".to_string()))?;

        w.scope(|w| {
            let overrode = cmd.override_destination(&self.config.destinations);
            for (name, destination) in &self.config.destinations {
                write!(w, "{}{} ", name, console::Emoji("🌐", ":"))?;
                match overrode.get(name) {
                    Some(overridden) if overridden != destination => {
                        writeln!(w, "{} {} {}", destination, console::Emoji("👉", "->"), overridden)?;
                    }
                    _ => {
                        writeln!(w, "{}", destination)?;
                    }
                }
            }
            Ok(())
        })?;

        w.scope(|w| {
            for outcome in &self.outcome {
                outcome.write(w, cmd)?;
            }
            Ok(())
        })?;
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
    pub fn write<T: std::io::Write>(&self, w: &mut OutcomeWriter<T>, cmd: &Assault) -> std::fmt::Result {
        let side = if self.pass() { console::Emoji("✅", "PASS") } else { console::Emoji("❌", "FAIL") };
        let target = console::style(&self.testcase.target);
        write!(w, "{} {} ", side, if self.pass() { target.green() } else { target.red() })?;
        if let Some(ref description) = self.testcase.description {
            writeln!(w, "{} {}", console::Emoji("📝", ""), description)?;
        } else {
            writeln!(w)?;
        }
        if !self.pass() && self.allow(cmd.strict) {
            w.scope(|w| {
                writeln!(w, "{} {}", console::Emoji("👟", ""), console::style("this testcase is allowed").green())
            })?;
        }
        Ok(())
    }
}

pub struct OutcomeWriter<T> {
    pub indent: usize,
    pub buf: T,
    pub at_start_line: bool,
}
impl OutcomeWriter<std::io::BufWriter<std::io::Stdout>> {
    pub fn with_stdout(indent: usize) -> Self {
        let buf = std::io::BufWriter::new(std::io::stdout());
        Self::new(indent, buf)
    }
}
impl<T> OutcomeWriter<T> {
    pub fn new(indent: usize, buf: T) -> Self {
        let at_start_line = true;
        Self { indent, buf, at_start_line }
    }
    pub fn indent(&self) -> String {
        "  ".repeat(self.indent)
    }
    pub fn increment(&mut self) {
        self.indent += 1;
    }
    pub fn decrement(&mut self) {
        self.indent -= 1;
    }
    pub fn scope<F, R, E>(&mut self, f: F) -> Result<R, E>
    where
        F: FnOnce(&mut Self) -> Result<R, E>,
        std::fmt::Error: From<E>,
    {
        self.increment();
        let ret = f(self)?;
        self.decrement();
        Ok(ret)
    }
}
impl<T: std::io::Write> std::fmt::Write for OutcomeWriter<T> {
    fn write_str(&mut self, s: &str) -> std::fmt::Result {
        // TODO better indent implementation ?
        if s.contains('\n') {
            for line in s.lines() {
                if self.at_start_line {
                    write!(self.buf, "{}", self.indent()).map_err(|_| std::fmt::Error)?;
                    self.at_start_line = false;
                }
                writeln!(self.buf, "{}", line).map_err(|_| std::fmt::Error)?;
                self.at_start_line = true;
            }
        } else {
            if self.at_start_line {
                write!(self.buf, "{}", self.indent()).map_err(|_| std::fmt::Error)?;
                self.at_start_line = false;
            }
            write!(self.buf, "{}", s).map_err(|_| std::fmt::Error)?;
        }
        Ok(())
    }
}
impl<T: Display> Display for OutcomeWriter<T> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.buf)
    }
}
