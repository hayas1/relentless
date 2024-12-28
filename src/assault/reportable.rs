use std::{
    fmt::{Display, Formatter, Write as FmtWrite},
    io::{BufWriter, Stdout, Write as IoWrite},
    process::ExitCode,
};

use crate::interface::{
    command::{Relentless, ReportFormat},
    config::{Setting, Testcase, WorkerConfig},
    helper::{
        coalesce::{Coalesce, Coalesced},
        http_serde_priv,
    },
};

use super::{
    destinations::Destinations,
    measure::aggregate::{Aggregate, EvaluateAggregator},
    messages::Messages,
};

pub trait Reportable {
    // TODO https://std-dev-guide.rust-lang.org/policy/specialization.html
    fn sub_reportable(&self) -> Option<Vec<&dyn Reportable>>;
    fn pass(&self) -> bool {
        if let Some(s) = self.sub_reportable() {
            s.iter().all(|r| r.pass())
        } else {
            true
        }
    }
    fn allow(&self, strict: bool) -> bool {
        if let Some(s) = self.sub_reportable() {
            s.iter().all(|r| r.allow(strict))
        } else {
            true
        }
    }
    fn aggregator(&self) -> EvaluateAggregator {
        if let Some(s) = self.sub_reportable() {
            s.iter().map(|r| r.aggregator()).fold(EvaluateAggregator::default(), |mut agg, b| {
                agg.merge(&b);
                agg
            })
        } else {
            EvaluateAggregator::default()
        }
    }
    fn skip_report(&self, cmd: &Relentless) -> bool {
        let Relentless { strict, ng_only, report_format, .. } = cmd;
        self.sub_reportable().map(|s| s.is_empty()).unwrap_or(false)
            || matches!(report_format, ReportFormat::NullDevice)
            || *ng_only && self.allow(*strict)
    }
}

/// TODO document
#[derive(Debug, Clone)]
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
    fn sub_reportable(&self) -> Option<Vec<&dyn Reportable>> {
        Some(self.report.iter().map(|r| r as _).collect())
    }
}

/// TODO document
#[derive(Debug, Clone)]
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
    fn sub_reportable(&self) -> Option<Vec<&dyn Reportable>> {
        Some(self.report.iter().map(|r| r as _).collect())
    }
}

/// TODO document
#[derive(Debug, Clone)]
pub struct CaseReport<T, Q, P> {
    testcase: Coalesced<Testcase<Q, P>, Setting<Q, P>>,
    pub(crate) passed: usize,
    messages: Messages<T>,
    aggregate: EvaluateAggregator,
}
impl<T, Q, P> CaseReport<T, Q, P> {
    pub fn new(
        testcase: Coalesced<Testcase<Q, P>, Setting<Q, P>>,
        passed: usize,
        messages: Messages<T>,
        aggregate: EvaluateAggregator,
    ) -> Self {
        Self { testcase, passed, messages, aggregate }
    }

    pub fn testcase(&self) -> &Coalesced<Testcase<Q, P>, Setting<Q, P>> {
        &self.testcase
    }
    pub fn messages(&self) -> &Messages<T> {
        &self.messages
    }
}
impl<T, Q: Clone + Coalesce, P: Clone + Coalesce> Reportable for CaseReport<T, Q, P> {
    fn sub_reportable(&self) -> Option<Vec<&dyn Reportable>> {
        None
    }
    fn pass(&self) -> bool {
        self.passed == self.testcase.coalesce().setting.repeat.times()
    }
    fn allow(&self, strict: bool) -> bool {
        let allowed = matches!(self.testcase.coalesce().setting.allow, Some(true));
        self.pass() || !strict && allowed
    }
    fn aggregator(&self) -> EvaluateAggregator {
        self.aggregate.clone()
    }
}

pub struct ReportWriter<W> {
    indent: usize,
    buf: W,
    at_start_line: bool,
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
