use std::{
    fmt::{Display, Formatter, Write as _},
    process::ExitCode,
};

use crate::{
    command::Relentless,
    config::{Coalesced, Destinations, Setting, Testcase, WorkerConfig},
    error::{Wrap, WrappedResult},
};

/// TODO document
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Outcome {
    outcome: Vec<WorkerOutcome>,
}
// TODO trait ?
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
    pub fn exit_code(&self, cmd: Relentless) -> ExitCode {
        (!self.allow(cmd.strict) as u8).into()
    }
    pub fn report(&self, cmd: &Relentless) -> WrappedResult<()> {
        self.report_to(&mut OutcomeWriter::with_stdout(0), cmd)
    }
    pub fn report_to<T: std::io::Write>(&self, w: &mut OutcomeWriter<T>, cmd: &Relentless) -> WrappedResult<()> {
        for outcome in &self.outcome {
            if !outcome.skip_report(cmd) {
                outcome.report_to(w, cmd)?;
                writeln!(w)?;
            }
        }
        Ok(())
    }
}

/// TODO document
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorkerOutcome {
    config: Coalesced<WorkerConfig, Destinations<String>>,
    outcome: Vec<CaseOutcome>,
}
impl WorkerOutcome {
    pub fn new(config: Coalesced<WorkerConfig, Destinations<String>>, outcome: Vec<CaseOutcome>) -> Self {
        Self { config, outcome }
    }
    pub fn pass(&self) -> bool {
        self.outcome.iter().all(|o| o.pass())
    }
    pub fn allow(&self, strict: bool) -> bool {
        self.outcome.iter().all(|o| o.allow(strict))
    }
    pub fn skip_report(&self, cmd: &Relentless) -> bool {
        let Relentless { strict, ng_only, no_report, .. } = cmd;
        *no_report || *ng_only && self.allow(*strict)
    }

    pub fn report_to<T: std::io::Write>(&self, w: &mut OutcomeWriter<T>, cmd: &Relentless) -> WrappedResult<()> {
        let WorkerConfig { name, destinations, .. } = self.config.coalesce();

        let side = console::Emoji("🚀", "");
        writeln!(w, "{} {} {}", side, name.as_ref().unwrap_or(&"testcases".to_string()), side)?;

        w.scope(|w| {
            for (name, destination) in destinations {
                write!(w, "{}{} ", name, console::Emoji("🌐", ":"))?;
                match self.config.base().destinations.get(&name) {
                    Some(base) if base != &destination => {
                        writeln!(w, "{} {} {}", base, console::Emoji("👉", "->"), destination)?;
                    }
                    _ => {
                        writeln!(w, "{}", destination)?;
                    }
                }
            }
            Ok::<_, Wrap>(())
        })?;

        w.scope(|w| {
            for outcome in &self.outcome {
                if !outcome.skip_report(cmd) {
                    outcome.report_to(w, cmd)?;
                }
            }
            Ok::<_, Wrap>(())
        })?;
        Ok(())
    }
}

/// TODO document
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CaseOutcome {
    testcase: Coalesced<Testcase, Setting>,
    passed: usize,
    pass: bool,
}
impl CaseOutcome {
    pub fn new(testcase: Coalesced<Testcase, Setting>, passed: usize) -> Self {
        let pass = passed == testcase.coalesce().setting.repeat.unwrap_or(1); // TODO here ?
        Self { testcase, passed, pass }
    }
    pub fn pass(&self) -> bool {
        self.pass
    }
    pub fn allow(&self, strict: bool) -> bool {
        let allowed = self.testcase.coalesce().attr.allow;
        self.pass() || !strict && allowed
    }
    pub fn skip_report(&self, cmd: &Relentless) -> bool {
        let Relentless { strict, ng_only, no_report, .. } = cmd;
        *no_report || *ng_only && self.allow(*strict)
    }

    pub fn report_to<T: std::io::Write>(&self, w: &mut OutcomeWriter<T>, cmd: &Relentless) -> WrappedResult<()> {
        let Testcase { description, target, setting, .. } = self.testcase.coalesce();

        let side = if self.pass() { console::Emoji("✅", "PASS") } else { console::Emoji("❌", "FAIL") };
        let target = console::style(&target);
        write!(w, "{} {} ", side, if self.pass() { target.green() } else { target.red() })?;
        if let Some(ref repeat) = setting.repeat {
            write!(w, "{}{}/{} ", console::Emoji("🔁", ""), self.passed, repeat)?;
        }
        if let Some(ref description) = description {
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
        Wrap: From<E>, // TODO remove wrap constraints
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
