use std::{
    fmt::{Display, Formatter, Write as _},
    process::ExitCode,
};

use bytes::Bytes;
use http_body::Body;
use http_body_util::BodyExt;
use serde_json::Value;

use crate::{
    command::Relentless,
    config::{Coalesced, Destinations, Evaluate, JsonEvaluate, Setting, Testcase, WorkerConfig},
    error::{FormatError, HttpError, RelentlessError},
};

#[allow(async_fn_in_trait)] // TODO #[warn(async_fn_in_trait)] by default
pub trait Evaluator<Res> {
    type Error;
    async fn evaluate(cfg: Option<&Evaluate>, res: Destinations<Res>) -> Result<bool, Self::Error>;
}
pub enum DefaultEvaluator {}
impl<ResB: Body> Evaluator<http::Response<ResB>> for DefaultEvaluator {
    type Error = RelentlessError;
    async fn evaluate(cfg: Option<&Evaluate>, res: Destinations<http::Response<ResB>>) -> Result<bool, Self::Error> {
        if res.len() == 1 {
            Self::status(res).await
        } else if !cfg!(feature = "json") {
            Self::compare(cfg, res).await
        } else {
            Self::compare_json(cfg, res).await
        }
    }
}

impl DefaultEvaluator {
    pub async fn parts<ResB: Body>(
        res: Destinations<http::Response<ResB>>,
    ) -> Result<
        Destinations<(http::StatusCode, http::HeaderMap, Bytes)>,
        <Self as Evaluator<http::Response<ResB>>>::Error,
    > {
        let mut d = Destinations::new();
        for (name, r) in res {
            let (http::response::Parts { status, headers, .. }, body) = r.into_parts();
            let bytes =
                BodyExt::collect(body).await.map(|buf| buf.to_bytes()).map_err(|_| HttpError::CannotConvertBody)?;
            d.insert(name, (status, headers, bytes));
        }
        Ok(d)
    }
    pub async fn status<ResB: Body>(
        res: Destinations<http::Response<ResB>>,
    ) -> Result<bool, <Self as Evaluator<http::Response<ResB>>>::Error> {
        Ok(res.into_iter().all(|(_name, res)| res.status().is_success()))
    }
    pub async fn compare<ResB: Body>(
        _cfg: Option<&Evaluate>,
        res: Destinations<http::Response<ResB>>,
    ) -> Result<bool, <Self as Evaluator<http::Response<ResB>>>::Error> {
        let v: Vec<_> = Self::parts(res).await?.into_values().collect();
        let pass = v.windows(2).all(|w| w[0] == w[1]);
        Ok(pass)
    }
}

#[cfg(feature = "json")]
impl DefaultEvaluator {
    pub async fn compare_json<ResB: Body>(
        cfg: Option<&Evaluate>,
        res: Destinations<http::Response<ResB>>,
    ) -> Result<bool, <Self as Evaluator<http::Response<ResB>>>::Error> {
        let v: Vec<_> = Self::parts(res).await?.into_values().collect();
        let pass = v.windows(2).all(|w| match Self::json_acceptable::<ResB>(cfg, (&w[0].2, &w[1].2)) {
            Ok(accept) => w[0].0 == w[1].0 && w[0].1 == w[1].1 && accept,
            Err(_) => w[0] == w[1],
        });
        Ok(pass)
    }

    pub fn json_acceptable<ResB: Body>(
        cfg: Option<&Evaluate>,
        (a, b): (&Bytes, &Bytes),
    ) -> Result<bool, <Self as Evaluator<http::Response<ResB>>>::Error> {
        let pointers = Self::pointers(&Self::diff(a, b)?);
        let ignored = pointers.iter().all(|op| {
            cfg.map(|c| match c {
                Evaluate::PlainText(_) => Vec::new(),
                Evaluate::Json(JsonEvaluate { ignore, .. }) => ignore.clone(),
            })
            .unwrap_or_default()
            .contains(op)
        });
        Ok(ignored)
    }

    pub fn diff(a: &Bytes, b: &Bytes) -> Result<json_patch::Patch, FormatError> {
        let (left, right): (serde_json::Value, serde_json::Value) =
            (serde_json::from_slice(a)?, serde_json::from_slice(b)?);
        Ok(json_patch::diff(&left, &right))
    }

    pub fn pointers(p: &json_patch::Patch) -> Vec<String> {
        // TODO implemented ?
        p.iter()
            .map(|op| match op {
                json_patch::PatchOperation::Add(json_patch::AddOperation { path, .. }) => path,
                json_patch::PatchOperation::Remove(json_patch::RemoveOperation { path, .. }) => path,
                json_patch::PatchOperation::Replace(json_patch::ReplaceOperation { path, .. }) => path,
                json_patch::PatchOperation::Move(json_patch::MoveOperation { path, .. }) => path,
                json_patch::PatchOperation::Copy(json_patch::CopyOperation { path, .. }) => path,
                json_patch::PatchOperation::Test(json_patch::TestOperation { path, .. }) => path,
            })
            .map(ToString::to_string)
            .collect()
    }
}

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
    pub fn report(&self, cmd: &Relentless) -> std::fmt::Result {
        self.report_to(&mut OutcomeWriter::with_stdout(0), cmd)
    }
    pub fn report_to<T: std::io::Write>(&self, w: &mut OutcomeWriter<T>, cmd: &Relentless) -> std::fmt::Result {
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

    pub fn report_to<T: std::io::Write>(&self, w: &mut OutcomeWriter<T>, cmd: &Relentless) -> std::fmt::Result {
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
            Ok::<_, std::fmt::Error>(())
        })?;

        w.scope(|w| {
            for outcome in &self.outcome {
                if !outcome.skip_report(cmd) {
                    outcome.report_to(w, cmd)?;
                }
            }
            Ok::<_, std::fmt::Error>(())
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

    pub fn report_to<T: std::io::Write>(&self, w: &mut OutcomeWriter<T>, cmd: &Relentless) -> std::fmt::Result {
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
