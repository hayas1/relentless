use std::fmt::{Display, Write as _};

use crate::{
    assault::{
        measure::{
            aggregate::{Aggregate, EvaluateAggregate, LatencyAggregate, PassAggregate, ResponseAggregate},
            threshold::{Classification, Classified, Classify},
        },
        reportable::{CaseReport, Report, ReportWriter, Reportable, WorkerReport},
    },
    interface::{
        command::{Relentless, WorkerKind},
        config::{Repeat, Testcase, WorkerConfig},
        helper::coalesce::Coalesce,
    },
};

pub trait ConsoleReport: Reportable {
    fn console_report<W: std::io::Write>(
        &self,
        cmd: &Relentless,
        w: &mut ReportWriter<W>,
    ) -> Result<(), std::fmt::Error>;
    fn console_aggregate<W: std::io::Write>(
        &self,
        cmd: &Relentless,
        w: &mut ReportWriter<W>,
    ) -> Result<(), std::fmt::Error> {
        let EvaluateAggregate { pass: pass_agg, response } = self.aggregator().aggregate(&cmd.quantile_set());
        let PassAggregate { pass, count, pass_rate } = &pass_agg;
        let ResponseAggregate { req, duration, rps, latency, .. } = &response;
        let LatencyAggregate { min, mean, quantile, max } = &latency;

        write!(
            w,
            "pass-rt: {}/{}={:.2}{}",
            pass,
            count,
            pass_agg.classify().apply_style(pass_rate * 100.),
            pass_agg.classify().apply_style("%"),
        )?;
        write!(w, "    ")?;
        writeln!(
            w,
            "rps: {}req/{:.2?}={:.2}{}",
            req,
            duration,
            response.classify().apply_style(rps),
            response.classify().apply_style("req/s"),
        )?;

        write!(w, "latency: min={:.3?} mean={:.3?} ", min.classified().styled(), mean.classified().styled())?;
        for (percentile, quantile) in cmd.percentile_set().iter().zip(quantile) {
            write!(w, "p{}={:.3?} ", percentile, quantile.classified().styled())?;
        }
        writeln!(w, "max={:.3?}", max.classified().styled())?;

        Ok(())
    }
}

pub enum RelentlessConsoleReport {}
impl RelentlessConsoleReport {
    pub const NAME_DEFAULT: &str = "configs";

    pub const SUMMARY_EMOJI: console::Emoji<'_, '_> = console::Emoji("💥", "");
}
impl<T: Display, Q: Clone + Coalesce, P: Clone + Coalesce> ConsoleReport for Report<T, Q, P> {
    fn console_report<W: std::io::Write>(
        &self,
        cmd: &Relentless,
        w: &mut ReportWriter<W>,
    ) -> Result<(), std::fmt::Error> {
        for report in &self.report {
            if !report.skip_report(cmd) {
                report.console_report(cmd, w)?;
                writeln!(w)?;
            }
        }

        if cmd.is_measure(WorkerKind::Configs) {
            writeln!(
                w,
                "{} {} {}",
                RelentlessConsoleReport::SUMMARY_EMOJI,
                console::style("summery of all requests in configs").bold(),
                RelentlessConsoleReport::SUMMARY_EMOJI,
            )?;
            w.scope(|w| self.console_aggregate(cmd, w))?;
        }

        Ok(())
    }
}

pub enum WorkerConsoleReport {}
impl WorkerConsoleReport {
    pub const NAME_DEFAULT: &'_ str = "testcases";
    pub const NAME_EMOJI: console::Emoji<'_, '_> = console::Emoji("🚀", "");
    pub const DESTINATION_EMOJI: console::Emoji<'_, '_> = console::Emoji("🌐", ":");
    pub const OVERWRITE_DESTINATION_EMOJI: console::Emoji<'_, '_> = console::Emoji("👉", "->");

    pub const SUMMARY_EMOJI: console::Emoji<'_, '_> = console::Emoji("💥", "");
}
impl<T: Display, Q: Clone + Coalesce, P: Clone + Coalesce> ConsoleReport for WorkerReport<T, Q, P> {
    fn console_report<W: std::io::Write>(
        &self,
        cmd: &Relentless,
        w: &mut ReportWriter<W>,
    ) -> Result<(), std::fmt::Error> {
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
            write!(w, "") // TODO type annotations needed
        })?;

        w.scope(|w| {
            for report in &self.report {
                if !report.skip_report(cmd) {
                    report.console_report(cmd, w)?;
                }
            }
            write!(w, "") // TODO type annotations needed
        })?;

        if cmd.is_measure(WorkerKind::Testcases) {
            w.scope(|w| {
                writeln!(
                    w,
                    "{} {}",
                    WorkerConsoleReport::SUMMARY_EMOJI,
                    console::style("summery of all requests in testcases").bold(),
                )?;
                w.scope(|w| self.console_aggregate(cmd, w))
            })?;
        }

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

    pub const SUMMARY_EMOJI: console::Emoji<'_, '_> = console::Emoji("💥", "");
}
impl<T: Display, Q: Clone + Coalesce, P: Clone + Coalesce> ConsoleReport for CaseReport<T, Q, P> {
    fn console_report<W: std::io::Write>(
        &self,
        cmd: &Relentless,
        w: &mut ReportWriter<W>,
    ) -> Result<(), std::fmt::Error> {
        let Testcase { description, target, setting, .. } = self.testcase().coalesce();

        let side = if self.pass() { CaseConsoleReport::PASS_EMOJI } else { CaseConsoleReport::FAIL_EMOJI };
        write!(w, "{} {} ", side, self.classify().apply_style(target))?;
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
                writeln!(
                    w,
                    "{} {}",
                    CaseConsoleReport::ALLOW_EMOJI,
                    Classification::Good.apply_style("this testcase is allowed")
                )
            })?;
        }
        if !self.messages().is_empty() {
            w.scope(|w| {
                writeln!(
                    w,
                    "{} {}",
                    CaseConsoleReport::MESSAGE_EMOJI,
                    self.messages().classify().apply_style("message was found")
                )?;
                w.scope(|w| {
                    let message = &self.messages();
                    writeln!(w, "{}", console::style(message).dim())
                })
            })?;
        }

        if cmd.is_measure(WorkerKind::Repeats) {
            w.scope(|w| {
                writeln!(
                    w,
                    "{} {}",
                    CaseConsoleReport::SUMMARY_EMOJI,
                    console::style("summery of all requests in repeats").bold(),
                )?;
                w.scope(|w| self.console_aggregate(cmd, w))
            })?;
        }

        Ok(())
    }
}

impl<T> Classified<T> {
    pub fn styled(&self) -> console::StyledObject<&T> {
        self.class().apply_style(&**self)
    }
}
impl Classification {
    pub fn style(&self) -> console::Style {
        match self {
            Classification::Good => console::Style::new().green(),
            Classification::Allow => console::Style::new().cyan(),
            Classification::Warn => console::Style::new().yellow(),
            Classification::Bad => console::Style::new().red(),
        }
    }
    pub fn apply_style<U>(&self, value: U) -> console::StyledObject<U> {
        self.style().apply_to(value)
    }
}
