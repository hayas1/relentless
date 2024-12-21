use std::fmt::{Display, Write as _};

use crate::{
    assault::{
        measure::aggregate::Aggregator,
        reportable::{CaseReport, Report, ReportWriter, Reportable, WorkerReport},
    },
    error::Wrap,
    interface::{
        command::Relentless,
        config::{Repeat, Testcase, WorkerConfig},
        helper::coalesce::Coalesce,
    },
};

pub trait ConsoleReport: Reportable {
    type Error;
    fn console_report<W: std::io::Write>(&self, cmd: &Relentless, w: &mut ReportWriter<W>) -> Result<(), Self::Error>;
}

impl<T: Display, Q: Clone + Coalesce, P: Clone + Coalesce> ConsoleReport for Report<T, Q, P> {
    type Error = crate::Error;
    fn console_report<W: std::io::Write>(&self, cmd: &Relentless, w: &mut ReportWriter<W>) -> Result<(), Self::Error> {
        for report in &self.report {
            if !report.skip_report(cmd) {
                report.console_report(cmd, w)?;
                writeln!(w).map_err(Wrap::wrapping)?;
            }
        }

        let ((passed, count, pass_rate), requests) = self.aggregate().aggregate();
        let (req, duration, rps, _bytes, latency) = requests;
        let (min, mean, quantile, max) = latency;
        write!(w, "passed: {}/{} ({:.2}%)\t", passed, count, pass_rate * 100.).map_err(Wrap::wrapping)?;
        writeln!(
            w,
            "rps: {}/{:?} ({:.2}req/s)",
            req,
            duration.unwrap_or_else(|_| todo!()),
            rps.unwrap_or_else(|_| todo!())
        )
        .map_err(Wrap::wrapping)?;
        writeln!(
            w,
            "min={:.3?} mean={:.3?} p50={:.3?} p90={:.3?} p99={:.3?} max={:.3?}",
            min, mean, quantile[0], quantile[1], quantile[2], max
        )
        .map_err(Wrap::wrapping)?;
        Ok(())
    }
}

pub enum WorkerConsoleReport {}
impl WorkerConsoleReport {
    pub const NAME_DEFAULT: &'_ str = "testcases";
    pub const NAME_EMOJI: console::Emoji<'_, '_> = console::Emoji("🚀", "");
    pub const DESTINATION_EMOJI: console::Emoji<'_, '_> = console::Emoji("🌐", ":");
    pub const OVERWRITE_DESTINATION_EMOJI: console::Emoji<'_, '_> = console::Emoji("👉", "->");
}

impl<T: Display, Q: Clone + Coalesce, P: Clone + Coalesce> ConsoleReport for WorkerReport<T, Q, P> {
    type Error = Wrap; // TODO crate::Error ?
    fn console_report<W: std::io::Write>(&self, cmd: &Relentless, w: &mut ReportWriter<W>) -> Result<(), Self::Error> {
        let WorkerConfig { name, destinations, .. } = self.config.coalesce();

        writeln!(
            w,
            "{} {} {}",
            WorkerConsoleReport::NAME_EMOJI,
            name.as_ref().unwrap_or(&WorkerConsoleReport::NAME_DEFAULT.to_string()),
            WorkerConsoleReport::NAME_EMOJI
        )?;

        w.scope(|w| {
            for (name, destination) in destinations {
                write!(w, "{}{} ", name, WorkerConsoleReport::DESTINATION_EMOJI)?;
                match self.config.base().destinations.get(&name) {
                    Some(base) if base != &destination => {
                        writeln!(
                            w,
                            "{} {} {}",
                            **base,
                            WorkerConsoleReport::OVERWRITE_DESTINATION_EMOJI,
                            *destination
                        )?;
                    }
                    _ => {
                        writeln!(w, "{}", *destination)?;
                    }
                }
            }
            Ok::<_, Wrap>(())
        })?;

        w.scope(|w| {
            for report in &self.report {
                if !report.skip_report(cmd) {
                    report.console_report(cmd, w)?;
                }
            }
            Ok::<_, Wrap>(())
        })?;
        Ok(())
    }
}

pub enum CaseConsoleReport {}
impl CaseConsoleReport {
    pub const PASS_EMOJI: console::Emoji<'_, '_> = console::Emoji("✅", "PASS");
    pub const FAIL_EMOJI: console::Emoji<'_, '_> = console::Emoji("❌", "FAIL");
    pub const REPEAT_EMOJI: console::Emoji<'_, '_> = console::Emoji("🔁", "");
    pub const DESCRIPTION_EMOJI: console::Emoji<'_, '_> = console::Emoji("📝", "");
    pub const ALLOW_EMOJI: console::Emoji<'_, '_> = console::Emoji("👀", "");
    pub const MESSAGE_EMOJI: console::Emoji<'_, '_> = console::Emoji("💬", "");
}

impl<T: Display, Q: Clone + Coalesce, P: Clone + Coalesce> ConsoleReport for CaseReport<T, Q, P> {
    type Error = Wrap; // TODO crate::Error ?
    fn console_report<W: std::io::Write>(&self, cmd: &Relentless, w: &mut ReportWriter<W>) -> Result<(), Self::Error> {
        let Testcase { description, target, setting, .. } = self.testcase().coalesce();

        let side = if self.pass() { CaseConsoleReport::PASS_EMOJI } else { CaseConsoleReport::FAIL_EMOJI };
        let target = console::style(&target);
        write!(w, "{} {} ", side, if self.pass() { target.green() } else { target.red() })?;
        if let Repeat(Some(ref repeat)) = setting.repeat {
            write!(w, "{}{}/{} ", CaseConsoleReport::REPEAT_EMOJI, self.passed, repeat)?;
        }
        if let Some(ref description) = description {
            writeln!(w, "{} {}", CaseConsoleReport::DESCRIPTION_EMOJI, description)?;
        } else {
            writeln!(w)?;
        }
        if !self.pass() && self.allow(cmd.strict) {
            w.scope(|w| {
                writeln!(w, "{} {}", CaseConsoleReport::ALLOW_EMOJI, console::style("this testcase is allowed").green())
            })?;
        }
        if !self.messages().is_empty() {
            w.scope(|w| {
                writeln!(w, "{} {}", CaseConsoleReport::MESSAGE_EMOJI, console::style("message was found").yellow())?;
                w.scope(|w| {
                    let message = &self.messages();
                    writeln!(w, "{}", console::style(message).dim())
                })
            })?;
        }
        Ok(())
    }
}
