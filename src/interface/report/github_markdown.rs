use std::fmt::{Display, Write as _};

use crate::{
    assault::report::{CaseReport, Report, ReportWriter, Reportable, WorkerReport},
    error::Wrap,
    interface::{
        command::Relentless,
        config::{Coalesce, Repeat, Testcase, WorkerConfig},
    },
};

pub trait GithubMarkdownReport: Reportable {
    type Error;
    fn github_markdown_report<W: std::io::Write>(
        &self,
        cmd: &Relentless,
        w: &mut ReportWriter<W>,
    ) -> Result<(), Self::Error>;
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
        let Testcase { description, target, setting, .. } = self.testcase.coalesce();

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
        if !self.messages.is_empty() {
            w.scope(|w| {
                writeln!(w, "<details>")?;
                w.scope(|w| {
                    writeln!(w, "<summary> {} message was found </summary>", CaseGithubMarkdownReport::MESSAGE_EMOJI)?;
                    writeln!(w)?;
                    writeln!(w, "```")?;
                    writeln!(w, "{}", &self.messages)?;
                    writeln!(w, "```")
                })?;
                writeln!(w, "</details>")
            })?;
        }
        Ok(())
    }
}
