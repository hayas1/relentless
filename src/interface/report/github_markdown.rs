use std::fmt::{Display, Write as _};

use crate::{
    assault::{
        measure::aggregate::{Aggregate, EvaluateAggregate, LatencyAggregate, PassAggregate, ResponseAggregate},
        reportable::{CaseReport, Report, ReportWriter, Reportable, WorkerReport},
    },
    error::Wrap,
    interface::{
        command::{Relentless, WorkerKind},
        config::{Repeat, Testcase, WorkerConfig},
        helper::coalesce::Coalesce,
    },
};

pub trait GithubMarkdownReport: Reportable {
    type Error;
    fn github_markdown_report<W: std::io::Write>(
        &self,
        cmd: &Relentless,
        w: &mut ReportWriter<W>,
    ) -> Result<(), Self::Error>;
    fn console_aggregate<W: std::io::Write, F: Fn(std::fmt::Error) -> Self::Error + Clone>(
        &self,
        cmd: &Relentless,
        w: &mut ReportWriter<W>,
        e: F, // TODO where Self::Error: From<std::io::Error> ?
    ) -> Result<(), Self::Error> {
        let EvaluateAggregate { pass: pass_agg, response } = self.aggregator().aggregate(&cmd.quantile_set());
        let PassAggregate { pass, count, pass_rate } = &pass_agg;
        let ResponseAggregate { req, duration, rps, latency, .. } = &response;
        let LatencyAggregate { min, mean, quantile, max } = &latency;

        write!(w, "| | min | mean |").map_err(e.clone())?;
        for percentile in cmd.percentile_set() {
            write!(w, " p{} |", percentile).map_err(e.clone())?;
        }
        writeln!(w, " max |").map_err(e.clone())?;

        write!(w, "| --- | --- | --- |").map_err(e.clone())?;
        for _ in cmd.percentile_set() {
            write!(w, " --- |").map_err(e.clone())?;
        }
        writeln!(w, " --- |").map_err(e.clone())?;

        write!(w, "| latency | {:.3?} | {:.3?} |", min, mean).map_err(e.clone())?;
        for q in quantile {
            write!(w, " {:.3?} |", q).map_err(e.clone())?;
        }
        writeln!(w, " {:.3?} |", max).map_err(e.clone())?;

        writeln!(w).map_err(e.clone())?;
        write!(w, "pass rate: {}/{}={:.2}%, ", pass, count, pass_rate * 100.).map_err(e.clone())?;
        writeln!(w, "rps: {}req/{:.2?}={:.2}req/s", req, duration, rps).map_err(e.clone())?;

        Ok(())
    }
}

pub enum RelentlessGithubMarkdownReport {}
impl RelentlessGithubMarkdownReport {
    pub const NAME_DEFAULT: &str = "configs";

    pub const SUMMARY_EMOJI: &str = ":boom:";
}
impl<T: Display, Q: Clone + Coalesce, P: Clone + Coalesce> GithubMarkdownReport for Report<T, Q, P> {
    type Error = crate::Error;
    fn github_markdown_report<W: std::io::Write>(
        &self,
        cmd: &Relentless,
        w: &mut ReportWriter<W>,
    ) -> Result<(), Self::Error> {
        for report in &self.report {
            if !report.skip_report(cmd) {
                report.github_markdown_report(cmd, w)?;
                writeln!(w).map_err(Wrap::wrapping)?;
            }
        }

        if cmd.is_measure(WorkerKind::Configs) {
            writeln!(
                w,
                "## {} summery of all requests in configs {}",
                RelentlessGithubMarkdownReport::SUMMARY_EMOJI,
                RelentlessGithubMarkdownReport::SUMMARY_EMOJI,
            )
            .map_err(Wrap::wrapping)?;
            w.scope(|w| self.console_aggregate(cmd, w, Wrap::error))?;
        }

        Ok(())
    }
}

pub enum WorkerGithubMarkdownReport {}
impl WorkerGithubMarkdownReport {
    pub const NAME_DEFAULT: &str = "testcases";
    pub const NAME_EMOJI: &str = ":rocket:";
    pub const DESTINATION_EMOJI: &str = ":globe_with_meridians:";
    pub const OVERWRITE_DESTINATION_EMOJI: &str = ":point_right:";
}
impl<T: Display, Q: Clone + Coalesce, P: Clone + Coalesce> GithubMarkdownReport for WorkerReport<T, Q, P> {
    type Error = Wrap; // TODO crate::Error ?
    fn github_markdown_report<W: std::io::Write>(
        &self,
        cmd: &Relentless,
        w: &mut ReportWriter<W>,
    ) -> Result<(), Self::Error> {
        let WorkerConfig { name, destinations, .. } = self.config.coalesce();

        writeln!(
            w,
            "## {} {} {}",
            WorkerGithubMarkdownReport::NAME_EMOJI,
            name.as_ref().unwrap_or(&WorkerGithubMarkdownReport::NAME_DEFAULT.to_string()),
            WorkerGithubMarkdownReport::NAME_EMOJI
        )?;

        for (name, destination) in destinations {
            write!(w, "{} {} ", name, WorkerGithubMarkdownReport::DESTINATION_EMOJI)?;
            match self.config.base().destinations.get(&name) {
                Some(base) if base != &destination => {
                    writeln!(
                        w,
                        "{} {} {}",
                        **base,
                        WorkerGithubMarkdownReport::OVERWRITE_DESTINATION_EMOJI,
                        *destination
                    )?;
                }
                _ => {
                    writeln!(w, "{}", *destination)?;
                }
            }
        }

        for report in &self.report {
            if !report.skip_report(cmd) {
                report.github_markdown_report(cmd, w)?;
            }
        }

        if cmd.is_measure(WorkerKind::Testcases) {
            w.scope(|w| self.console_aggregate(cmd, w, Wrap::wrapping))?;
        }

        Ok(())
    }
}

pub enum CaseGithubMarkdownReport {}
impl CaseGithubMarkdownReport {
    pub const PASS_EMOJI: &str = ":white_check_mark:";
    pub const FAIL_EMOJI: &str = ":x:";
    pub const REPEAT_EMOJI: &str = ":repeat:";
    pub const DESCRIPTION_EMOJI: &str = ":memo:";
    pub const ALLOW_EMOJI: &str = ":eyes:";
    pub const MESSAGE_EMOJI: &str = ":speech_balloon:";
}
impl<T: Display, Q: Clone + Coalesce, P: Clone + Coalesce> GithubMarkdownReport for CaseReport<T, Q, P> {
    type Error = Wrap; // TODO crate::Error ?
    fn github_markdown_report<W: std::io::Write>(
        &self,
        cmd: &Relentless,
        w: &mut ReportWriter<W>,
    ) -> Result<(), Self::Error> {
        let Testcase { description, target, setting, .. } = self.testcase().coalesce();

        let side =
            if self.pass() { CaseGithubMarkdownReport::PASS_EMOJI } else { CaseGithubMarkdownReport::FAIL_EMOJI };
        write!(w, "- {} `{}` ", side, target)?;
        if let Repeat(Some(ref repeat)) = setting.repeat {
            write!(w, "{}{}/{} ", CaseGithubMarkdownReport::REPEAT_EMOJI, self.passed, repeat)?;
        }
        if let Some(ref description) = description {
            writeln!(w, "{} {}", CaseGithubMarkdownReport::DESCRIPTION_EMOJI, description)?;
        } else {
            writeln!(w)?;
        }
        if !self.pass() && self.allow(cmd.strict) {
            w.scope(|w| writeln!(w, "{} this testcase is allowed", CaseGithubMarkdownReport::ALLOW_EMOJI))?;
        }
        if !self.messages().is_empty() {
            w.scope(|w| {
                writeln!(w, "<details>")?;
                w.scope(|w| {
                    writeln!(w, "<summary> {} message was found </summary>", CaseGithubMarkdownReport::MESSAGE_EMOJI)?;
                    writeln!(w)?;
                    writeln!(w, "```")?;
                    writeln!(w, "{}", &self.messages())?;
                    writeln!(w, "```")
                })?;
                writeln!(w, "</details>")
            })?;
        }

        if cmd.is_measure(WorkerKind::Repeats) {
            w.scope(|w| self.console_aggregate(cmd, w, Wrap::wrapping))?;
        }

        Ok(())
    }
}
