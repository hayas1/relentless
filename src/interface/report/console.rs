use std::{
    fmt::{Display, Write as _},
    time::Duration,
};

use crate::{
    assault::{
        measure::{
            aggregate::{Aggregate, EvaluateAggregate, LatencyAggregate, PassAggregate, ResponseAggregate},
            threshold::Classified,
        },
        reportable::{CaseReport, Report, ReportWriter, Reportable, WorkerReport},
    },
    error::Wrap,
    interface::{
        command::{Relentless, WorkerKind},
        config::{Repeat, Testcase, WorkerConfig},
        helper::coalesce::Coalesce,
    },
};

// TODO trait ? use classified for another style ?
pub fn style_classified(class: &Classified) -> console::Style {
    match class {
        Classified::Good => console::Style::new().green(),
        Classified::Allow => console::Style::new().cyan(),
        Classified::Warn => console::Style::new().yellow(),
        Classified::Bad => console::Style::new().red(),
    }
}
pub fn apply_style_classified(duration: &Duration) -> console::StyledObject<std::time::Duration> {
    style_classified(&Classified::latency(*duration)).apply_to(*duration)
}

pub trait ConsoleReport: Reportable {
    type Error;
    fn console_report<W: std::io::Write>(&self, cmd: &Relentless, w: &mut ReportWriter<W>) -> Result<(), Self::Error>;
    fn console_aggregate<W: std::io::Write, F: Fn(std::fmt::Error) -> Self::Error + Clone>(
        &self,
        cmd: &Relentless,
        w: &mut ReportWriter<W>,
        e: F, // TODO where Self::Error: From<std::io::Error> ?
    ) -> Result<(), Self::Error> {
        let EvaluateAggregate { pass: pass_agg, response } = self.aggregator().aggregate();
        let PassAggregate { pass, count, pass_rate } = &pass_agg;
        let ResponseAggregate { req, duration, rps, latency, .. } = &response;
        let LatencyAggregate { min, mean, quantile, max } = &latency;

        write!(
            w,
            "pass-rt: {}/{}={:.2}{}",
            pass,
            count,
            style_classified(&Classified::pass_agg(&pass_agg)).apply_to(pass_rate * 100.),
            style_classified(&Classified::pass_agg(&pass_agg)).apply_to("%"),
        )
        .map_err(e.clone())?;
        write!(w, "    ").map_err(e.clone())?;
        writeln!(
            w,
            "rps: {}req/{:.2?}={:.2}{}",
            req,
            duration.unwrap_or_default(),
            style_classified(&Classified::response_agg(&response)).apply_to(rps.unwrap_or_default()),
            style_classified(&Classified::response_agg(&response)).apply_to("req/s"),
        )
        .map_err(e.clone())?;

        write!(w, "latency: min={:.3?} mean={:.3?} ", apply_style_classified(min), apply_style_classified(mean),)
            .map_err(e.clone())?;
        for (percentile, quantile) in cmd.percentile_set().iter().zip(quantile) {
            write!(w, "p{}={:.3?} ", percentile, apply_style_classified(quantile)).map_err(e.clone())?;
        }
        writeln!(w, "max={:.3?}", apply_style_classified(max),).map_err(e.clone())?;

        Ok(())
    }
}

pub enum RelentlessConsoleReport {}
impl RelentlessConsoleReport {
    pub const NAME_DEFAULT: &str = "configs";

    pub const SUMMARY_EMOJI: console::Emoji<'_, '_> = console::Emoji("💥", "");
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

        if cmd.is_measure(WorkerKind::Configs) {
            writeln!(
                w,
                "{} {} {}",
                RelentlessConsoleReport::SUMMARY_EMOJI,
                console::style("summery of all requests in configs").bold(),
                RelentlessConsoleReport::SUMMARY_EMOJI,
            )
            .map_err(Wrap::wrapping)?;
            w.scope(|w| self.console_aggregate(cmd, w, Wrap::error))?;
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

        if cmd.is_measure(WorkerKind::Testcases) {
            w.scope(|w| {
                writeln!(
                    w,
                    "{} {}",
                    WorkerConsoleReport::SUMMARY_EMOJI,
                    console::style("summery of all requests in testcases").bold(),
                )
                .map_err(Wrap::wrapping)?;
                w.scope(|w| self.console_aggregate(cmd, w, Wrap::wrapping))
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

        if cmd.is_measure(WorkerKind::Repeats) {
            w.scope(|w| {
                writeln!(
                    w,
                    "{} {}",
                    CaseConsoleReport::SUMMARY_EMOJI,
                    console::style("summery of all requests in repeats").bold(),
                )
                .map_err(Wrap::wrapping)?;
                w.scope(|w| self.console_aggregate(cmd, w, Wrap::wrapping))
            })?;
        }

        Ok(())
    }
}
