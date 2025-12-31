use std::fmt::{Display, Write as _};

use console::{Emoji, Style, StyledObject};

use crate::{
    evaluator::evaluate::{Message, MessageKind},
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
    pub fn message_style(&self, kind: &MessageKind) -> Style {
        match kind {
            MessageKind::Warn => Style::new().yellow().dim(),
            MessageKind::Error => Style::new().red().dim(),
        }
    }
    pub fn styled_message<'a, T>(&self, msg: &'a Message<T>) -> StyledObject<&'a T> {
        self.message_style(&msg.kind).apply_to(&msg.message)
    }
}
impl<C, Q, P, M: Display> Reporter<&JobReport<'_, C, Q, P, M>> for Console {
    type Error = std::fmt::Error;
    fn write_report<W: std::io::Write>(
        &self,
        writer: &mut ReportWriter<W>,
        report: &JobReport<C, Q, P, M>,
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
impl<C, Q, P, M: Display> Reporter<&SuiteReport<'_, C, Q, P, M>> for Console {
    type Error = std::fmt::Error;
    fn write_report<W: std::io::Write>(
        &self,
        writer: &mut ReportWriter<W>,
        report: &SuiteReport<C, Q, P, M>,
    ) -> Result<(), Self::Error> {
        writeln!(writer, "{} {} {}", Self::SUITE_NAME_EMOJI, report.suite.name, Self::SUITE_NAME_EMOJI)?;
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
impl<Q, P, M: Display> Reporter<&CaseReport<'_, Q, P, M>> for Console {
    type Error = std::fmt::Error;
    fn write_report<W: std::io::Write>(
        &self,
        writer: &mut ReportWriter<W>,
        report: &CaseReport<Q, P, M>,
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
            let l2 = {
                let (mut lines, and_more) = report.messages.display_lines();
                lines.try_for_each(|l| {
                    write!(w, "{} ", Self::CASE_MESSAGE_EMOJI)?;
                    w.scope_n(3, |w| writeln!(w, "{}", self.styled_message(l)))
                })?;
                and_more.iter().try_for_each(|m| writeln!(w, "... and {m} more"))
            };
            l2
        })?;
        Ok(())
    }
}
