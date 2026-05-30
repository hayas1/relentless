use std::fmt::{Display, Write as _};

use crate::{
    report::{ReportSpec, ReportWriter, Reporter},
    shot::{contract::Assessment, job::JobReport, profile::Repeat, suite::SuiteReport, testcase::CaseReport},
};

pub struct GithubMarkdown<'a> {
    pub spec: &'a ReportSpec,
}
impl<'a> GithubMarkdown<'a> {
    pub const SUITE_NAME_EMOJI: &'static str = ":rocket:";
    pub const DESTINATION_EMOJI: &'static str = ":globe_with_meridians:";
    pub const OVERWRITE_DESTINATION_EMOJI: &'static str = ":point_right:";

    pub const CASE_PASS_EMOJI: &'static str = ":white_check_mark:";
    pub const CASE_FAIL_EMOJI: &'static str = ":x:";
    pub const CASE_ALLOW_EMOJI: &'static str = ":eyes:";
    pub const CASE_REPEAT_EMOJI: &'static str = ":repeat:";
    pub const CASE_DESCRIPTION_EMOJI: &'static str = ":memo:";
    pub const CASE_MESSAGE_EMOJI: &'static str = ":speech_balloon:";

    pub fn new(spec: &'a ReportSpec) -> Self {
        Self { spec }
    }
}

impl<C, Q, P, M: Display> Reporter<&JobReport<'_, C, Q, P, M>> for GithubMarkdown<'_> {
    type Error = std::fmt::Error;

    fn write_report<W: std::io::Write>(
        &self,
        writer: &mut ReportWriter<W>,
        report: &JobReport<C, Q, P, M>,
    ) -> Result<(), Self::Error> {
        report.suites.iter().try_for_each(|s| {
            self.write_report(writer, s)?;
            writeln!(writer)
        })?;

        let job = if report.evaluated.pass {
            "PASS"
        } else if report.evaluated.allow {
            "ALLOW"
        } else {
            "FAIL"
        };
        writeln!(writer, "job: {job}")
    }
}

impl<C, Q, P, M: Display> Reporter<&SuiteReport<'_, C, Q, P, M>> for GithubMarkdown<'_> {
    type Error = std::fmt::Error;

    fn write_report<W: std::io::Write>(
        &self,
        writer: &mut ReportWriter<W>,
        report: &SuiteReport<'_, C, Q, P, M>,
    ) -> Result<(), Self::Error> {
        writeln!(writer, "## {} {} {}", Self::SUITE_NAME_EMOJI, report.suite.name, Self::SUITE_NAME_EMOJI)?;
        let (first, last) = (report.destinations.first(), report.destinations.last());
        report.destinations.combine_rev_clone().iter().try_for_each(|(name, dest)| {
            write!(writer, "- {name} {} ", Self::DESTINATION_EMOJI)?;
            if let (Some(base), Some(overwrite)) = (first.get(name), last.get(name)) {
                writeln!(writer, "{} {} {}", base, Self::OVERWRITE_DESTINATION_EMOJI, overwrite)
            } else {
                writeln!(writer, "{dest}")
            }
        })?;
        writeln!(writer)?;

        report.cases.iter().try_for_each(|c| self.write_report(writer, c))
    }
}

impl<Q, P, M: Display> Reporter<&CaseReport<'_, Q, P, M>> for GithubMarkdown<'_> {
    type Error = std::fmt::Error;

    fn write_report<W: std::io::Write>(
        &self,
        writer: &mut ReportWriter<W>,
        report: &CaseReport<'_, Q, P, M>,
    ) -> Result<(), Self::Error> {
        let assessment = report.evaluated.assess();
        if self.spec.ng_only && assessment != Assessment::Bad {
            return Ok(());
        }

        let icon = match assessment {
            Assessment::Good => Self::CASE_PASS_EMOJI,
            Assessment::Acceptable | Assessment::Poor => Self::CASE_ALLOW_EMOJI,
            Assessment::Bad => Self::CASE_FAIL_EMOJI,
        };

        write!(writer, "{icon} `{}` ", report.case.target)?;
        if let Repeat(Some(repeat)) = &report.case.profile.repeat {
            write!(writer, "{} {}/{repeat} ", Self::CASE_REPEAT_EMOJI, report.evaluated.allowed)?;
        }
        if let Some(description) = &report.case.description {
            writeln!(writer, "{} {description}", Self::CASE_DESCRIPTION_EMOJI)?;
        } else {
            writeln!(writer)?;
        }

        if !report.messages.is_empty() {
            writer.scope(|w| {
                writeln!(w, "<details>")?;
                w.scope(|w| {
                    writeln!(w, "<summary> {} message was found </summary>", Self::CASE_MESSAGE_EMOJI)?;
                    writeln!(w)?;
                    writeln!(w, "```")?;
                    writeln!(w, "{}", &report.messages)?;
                    writeln!(w, "```")
                })?;
                writeln!(w, "</details>")?;
                writeln!(w)
            })?;
        }

        Ok(())
    }
}
