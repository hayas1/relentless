use std::{
    fmt::{Display, Formatter, Write as FmtWrite},
    io::{BufWriter, Stdout, Write as IoWrite},
    process::ExitCode,
};

use crate::{
    error::{MultiWrap, Wrap},
    interface::{
        command::{Relentless, ReportFormat},
        config::{Setting, Testcase, WorkerConfig},
        helper::{
            coalesce::{Coalesce, Coalesced},
            http_serde_priv,
        },
    },
    service::destinations::Destinations,
};

/// TODO document
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Report<T, Q, P> {
    pub report: Vec<WorkerReport<T, Q, P>>,
}
impl<T, Q: Clone + Coalesce, P: Clone + Coalesce> Report<T, Q, P> {
    pub fn new(report: Vec<WorkerReport<T, Q, P>>) -> Self {
        Self { report }
    }
    pub fn exit_code(&self, cmd: &Relentless) -> ExitCode {
        (!self.allow(cmd.strict) as u8).into()
    }
}
impl<T, Q: Clone + Coalesce, P: Clone + Coalesce> Reportable for Report<T, Q, P> {
    fn sub_reportable(&self) -> Vec<&dyn Reportable> {
        self.report.iter().map(|r| r as _).collect()
    }
}

/// TODO document
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorkerReport<T, Q, P> {
    pub config: Coalesced<WorkerConfig<Q, P>, Destinations<http_serde_priv::Uri>>,
    pub report: Vec<CaseReport<T, Q, P>>,
}
impl<T, Q, P> WorkerReport<T, Q, P> {
    pub fn new(
        config: Coalesced<WorkerConfig<Q, P>, Destinations<http_serde_priv::Uri>>,
        report: Vec<CaseReport<T, Q, P>>,
    ) -> Self {
        Self { config, report }
    }
}
impl<T, Q: Clone + Coalesce, P: Clone + Coalesce> Reportable for WorkerReport<T, Q, P> {
    fn sub_reportable(&self) -> Vec<&dyn Reportable> {
        self.report.iter().map(|r| r as _).collect()
    }
}

/// TODO document
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CaseReport<T, Q, P> {
    pub testcase: Coalesced<Testcase<Q, P>, Setting<Q, P>>,
    pub passed: usize,
    pub messages: MultiWrap<T>,
}
impl<T, Q, P> CaseReport<T, Q, P> {
    pub fn new(testcase: Coalesced<Testcase<Q, P>, Setting<Q, P>>, passed: usize, messages: MultiWrap<T>) -> Self {
        Self { testcase, passed, messages }
    }
}
impl<T, Q: Clone + Coalesce, P: Clone + Coalesce> Reportable for CaseReport<T, Q, P> {
    fn sub_reportable(&self) -> Vec<&dyn Reportable> {
        Vec::new()
    }
    fn pass(&self) -> bool {
        self.passed == self.testcase.coalesce().setting.repeat.times()
    }
    fn allow(&self, strict: bool) -> bool {
        let allowed = self.testcase.coalesce().attr.allow;
        self.pass() || !strict && allowed
    }
}

pub trait Reportable {
    // TODO https://std-dev-guide.rust-lang.org/policy/specialization.html
    fn sub_reportable(&self) -> Vec<&dyn Reportable>;
    fn pass(&self) -> bool {
        if self.sub_reportable().is_empty() {
            unreachable!("a reportable without children should implement its own method");
        } else {
            self.sub_reportable().iter().all(|r| r.pass())
        }
    }
    fn allow(&self, strict: bool) -> bool {
        if self.sub_reportable().is_empty() {
            unreachable!("a reportable without children should implement its own method");
        } else {
            self.sub_reportable().iter().all(|r| r.allow(strict))
        }
    }
    fn skip_report(&self, cmd: &Relentless) -> bool {
        let Relentless { strict, ng_only, report_format, .. } = cmd;
        matches!(report_format, ReportFormat::NullDevice) || *ng_only && self.allow(*strict)
    }
}

pub struct ReportWriter<W> {
    pub indent: usize,
    pub buf: W,
    pub at_start_line: bool,
}
impl ReportWriter<BufWriter<Stdout>> {
    pub fn with_stdout(indent: usize) -> Self {
        let buf = BufWriter::new(std::io::stdout());
        Self::new(indent, buf)
    }
}
impl<W> ReportWriter<W> {
    pub fn new(indent: usize, buf: W) -> Self {
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
impl<W: IoWrite> FmtWrite for ReportWriter<W> {
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
impl<W: Display> Display for ReportWriter<W> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.buf)
    }
}
