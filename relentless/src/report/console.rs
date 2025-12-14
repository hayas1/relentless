use std::fmt::Write as _;

use console::{Emoji, Style, StyledObject};

use crate::{
    report::{ReportWriter, Reporter},
    shot::{
        contract::{Assessment, Evaluated},
        job::JobReport,
        profile::Repeat,
        suite::SuiteReport,
        testcase::CaseReport,
    },
};

pub struct Console;
impl Console {
    pub const SUITE_NAME_EMOJI: Emoji<'_, '_> = Emoji("🚀", "");
    pub const SUITE_DESTINATION_EMOJI: Emoji<'_, '_> = Emoji("🌐", ":");
    pub const SUITE_OVERWRITE_DESTINATION_EMOJI: Emoji<'_, '_> = Emoji("👉", "->");

    pub const CASE_PASS_EMOJI: Emoji<'_, '_> = Emoji("✅", "PASS");
    pub const CASE_FAIL_EMOJI: Emoji<'_, '_> = Emoji("❌", "FAIL");
    pub const CASE_REPEAT_EMOJI: Emoji<'_, '_> = Emoji("🔁", "");
    pub const CASE_DESCRIPTION_EMOJI: Emoji<'_, '_> = Emoji("📝", "");
    pub const CASE_ALLOW_EMOJI: Emoji<'_, '_> = Emoji("👀", "");
    pub const CASE_MESSAGE_EMOJI: Emoji<'_, '_> = Emoji("💬", "");

    pub const SUMMARY_EMOJI: Emoji<'_, '_> = Emoji("💥", "");

    pub fn style(&self, assessment: &Assessment) -> Style {
        match assessment {
            Assessment::Good => Style::new().green(),
            Assessment::Acceptable => Style::new().cyan(),
            Assessment::Poor => Style::new().yellow(),
            Assessment::Bad => Style::new().red(),
        }
    }
    pub fn styled<T>(&self, value: T, assessment: &Assessment) -> StyledObject<T> {
        self.style(assessment).apply_to(value)
    }
}
impl<C, Q, P> Reporter<&JobReport<'_, C, Q, P>> for Console {
    type Error = std::fmt::Error;
    fn write_report<W: std::io::Write>(
        &self,
        writer: &mut ReportWriter<W>,
        report: &JobReport<C, Q, P>,
    ) -> Result<(), Self::Error> {
        report.suites.iter().try_fold((), |(), s| {
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
        writeln!(writer, "job: {job}")?;
        Ok(())
    }
}
impl<C, Q, P> Reporter<&SuiteReport<'_, C, Q, P>> for Console {
    type Error = std::fmt::Error;
    fn write_report<W: std::io::Write>(
        &self,
        writer: &mut ReportWriter<W>,
        report: &SuiteReport<C, Q, P>,
    ) -> Result<(), Self::Error> {
        writeln!(writer, "{} {}", Self::SUITE_NAME_EMOJI, report.suite.name)?;
        writer.scope(|w| {
            let (first, last) = (report.destinations.first(), report.destinations.last());
            report.destinations.combine_rev_clone().iter().try_fold((), |(), (name, dest)| {
                write!(w, "{name}{} ", Self::SUITE_DESTINATION_EMOJI)?;
                if let (Some(base), Some(overwrite)) = (first.get(name), last.get(name)) {
                    writeln!(w, "{base} {} {overwrite}", Self::SUITE_OVERWRITE_DESTINATION_EMOJI)
                } else {
                    writeln!(w, "{dest}")
                }
            })
        })?;
        report.cases.iter().try_fold((), |(), c| self.write_report(writer, c))
    }
}
impl<Q, P> Reporter<&CaseReport<'_, Q, P>> for Console {
    type Error = std::fmt::Error;
    fn write_report<W: std::io::Write>(
        &self,
        writer: &mut ReportWriter<W>,
        report: &CaseReport<Q, P>,
    ) -> Result<(), Self::Error> {
        let assessment = report.evaluated.assess();
        let Evaluated { pass, passed, allow, .. } = &report.evaluated;
        let l1 = {
            let icon = match (pass, allow) {
                (true, _) => self.styled(Self::CASE_PASS_EMOJI, &assessment),
                (false, true) => self.styled(Self::CASE_ALLOW_EMOJI, &assessment),
                (false, false) => self.styled(Self::CASE_FAIL_EMOJI, &assessment),
            };
            write!(writer, "{icon} {}", self.styled(&report.case.target, &assessment))?;
            if let Repeat(Some(repeat)) = &report.case.profile.repeat {
                write!(writer, " {}{passed}/{repeat}", Self::CASE_REPEAT_EMOJI)?;
            }
            if let Some(description) = &report.case.description {
                write!(writer, " {} {description}", Self::CASE_DESCRIPTION_EMOJI)?;
            }
            writeln!(writer)
        };
        l1?;
        writer.scope(|w| {
            let messages: Vec<String> = Vec::new();
            let l2 = {
                if !messages.is_empty() {
                    writeln!(w, "{} messages", Self::CASE_MESSAGE_EMOJI)?; // TODO usize scope indent
                    messages.iter().try_fold((), |(), m| writeln!(w, "{m}"))?;
                }
                Ok(())
            };
            l2?;
            Ok(())
        })?;
        Ok(())
    }
}
