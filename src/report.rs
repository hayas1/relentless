use std::fmt::{Display, Formatter};
use std::process::ExitCode;

use crate::error::Wrap;
use crate::{
    command::{Relentless, ReportFormat},
    config::{http_serde_priv, Coalesced, Destinations, Setting, Testcase, WorkerConfig},
    error::MultiWrap,
};

/// TODO document
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Report<T> {
    report: Vec<WorkerReport<T>>,
}
impl<T> Report<T> {
    pub fn new(report: Vec<WorkerReport<T>>) -> Self {
        Self { report }
    }
    pub fn exit_code(&self, cmd: &Relentless) -> ExitCode {
        (!self.allow(cmd.strict) as u8).into()
    }
}
impl<T> Reportable for Report<T> {
    fn sub_reportable(&self) -> Vec<&dyn Reportable> {
        self.report.iter().map(|r| r as _).collect()
    }
}

/// TODO document
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorkerReport<T> {
    config: Coalesced<WorkerConfig, Destinations<http_serde_priv::Uri>>,
    report: Vec<CaseReport<T>>,
}
impl<T> WorkerReport<T> {
    pub fn new(
        config: Coalesced<WorkerConfig, Destinations<http_serde_priv::Uri>>,
        report: Vec<CaseReport<T>>,
    ) -> Self {
        Self { config, report }
    }
}
impl<T> Reportable for WorkerReport<T> {
    fn sub_reportable(&self) -> Vec<&dyn Reportable> {
        self.report.iter().map(|r| r as _).collect()
    }
}

/// TODO document
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CaseReport<T> {
    testcases: Coalesced<Testcase, Setting>,
    passed: usize,
    messages: MultiWrap<T>,
}
impl<T> CaseReport<T> {
    pub fn new(testcases: Coalesced<Testcase, Setting>, passed: usize, messages: MultiWrap<T>) -> Self {
        Self { testcases, passed, messages }
    }
}
impl<T> Reportable for CaseReport<T> {
    fn sub_reportable(&self) -> Vec<&dyn Reportable> {
        Vec::new()
    }
    fn pass(&self) -> bool {
        self.passed == self.testcases.coalesce().setting.repeat.times()
    }
    fn allow(&self, strict: bool) -> bool {
        let allowed = self.testcases.coalesce().attr.allow;
        self.pass() || !strict && allowed
    }
}

pub trait Reportable {
    // TODO https://std-dev-guide.rust-lang.org/policy/specialization.html
    fn sub_reportable(&self) -> Vec<&dyn Reportable>;
    fn pass(&self) -> bool {
        if self.sub_reportable().is_empty() {
            unreachable!("a reportable without children should implement its own method");
        } else {
            self.sub_reportable().iter().all(|r| r.pass())
        }
    }
    fn allow(&self, strict: bool) -> bool {
        if self.sub_reportable().is_empty() {
            unreachable!("a reportable without children should implement its own method");
        } else {
            self.sub_reportable().iter().all(|r| r.allow(strict))
        }
    }
    fn skip_report(&self, cmd: &Relentless) -> bool {
        let Relentless { strict, ng_only, report_format, .. } = cmd;
        matches!(report_format, ReportFormat::NullDevice) || *ng_only && self.allow(*strict)
    }
}

pub struct ReportWriter<W> {
    pub indent: usize,
    pub buf: W,
    pub at_start_line: bool,
}
impl ReportWriter<std::io::BufWriter<std::io::Stdout>> {
    pub fn with_stdout(indent: usize) -> Self {
        let buf = std::io::BufWriter::new(std::io::stdout());
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
        Wrap: From<E>, // TODO remove wrap constraints
    {
        self.increment();
        let ret = f(self)?;
        self.decrement();
        Ok(ret)
    }
}
impl<W: std::io::Write> std::fmt::Write for ReportWriter<W> {
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

#[cfg(feature = "console-report")]
pub mod console_report {
    use std::fmt::{Display, Write as _};

    use crate::{
        command::Relentless,
        config::{Repeat, Testcase, WorkerConfig},
        error::Wrap,
    };

    use super::{CaseReport, Report, ReportWriter, Reportable, WorkerReport};

    pub trait ConsoleReport: Reportable {
        type Error;
        fn console_report<W: std::io::Write>(
            &self,
            cmd: &Relentless,
            w: &mut ReportWriter<W>,
        ) -> Result<(), Self::Error>;
    }

    impl<T: Display> ConsoleReport for Report<T> {
        type Error = crate::Error;
        fn console_report<W: std::io::Write>(
            &self,
            cmd: &Relentless,
            w: &mut ReportWriter<W>,
        ) -> Result<(), Self::Error> {
            for report in &self.report {
                if !report.skip_report(cmd) {
                    report.console_report(cmd, w)?;
                    writeln!(w).map_err(Wrap::wrapping)?;
                }
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
    }

    impl<T: Display> ConsoleReport for WorkerReport<T> {
        type Error = Wrap; // TODO crate::Error ?
        fn console_report<W: std::io::Write>(
            &self,
            cmd: &Relentless,
            w: &mut ReportWriter<W>,
        ) -> Result<(), Self::Error> {
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

    impl<T: Display> ConsoleReport for CaseReport<T> {
        type Error = Wrap; // TODO crate::Error ?
        fn console_report<W: std::io::Write>(
            &self,
            cmd: &Relentless,
            w: &mut ReportWriter<W>,
        ) -> Result<(), Self::Error> {
            let Testcase { description, target, setting, .. } = self.testcases.coalesce();

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
                    writeln!(
                        w,
                        "{} {}",
                        CaseConsoleReport::ALLOW_EMOJI,
                        console::style("this testcase is allowed").green()
                    )
                })?;
            }
            if !self.messages.is_empty() {
                w.scope(|w| {
                    writeln!(
                        w,
                        "{} {}",
                        CaseConsoleReport::MESSAGE_EMOJI,
                        console::style("message was found").yellow()
                    )?;
                    w.scope(|w| {
                        let message = &self.messages;
                        writeln!(w, "{}", console::style(message).dim())
                    })
                })?;
            }
            Ok(())
        }
    }
}

pub mod github_markdown_report {
    use std::fmt::{Display, Write as _};

    use crate::{
        command::Relentless,
        config::{Repeat, Testcase, WorkerConfig},
        error::Wrap,
    };

    use super::{CaseReport, Report, ReportWriter, Reportable, WorkerReport};

    pub trait GithubMarkdownReport: Reportable {
        type Error;
        fn github_markdown_report<W: std::io::Write>(
            &self,
            cmd: &Relentless,
            w: &mut ReportWriter<W>,
        ) -> Result<(), Self::Error>;
    }

    impl<T: Display> GithubMarkdownReport for Report<T> {
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

    impl<T: Display> GithubMarkdownReport for WorkerReport<T> {
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

    impl<T: Display> GithubMarkdownReport for CaseReport<T> {
        type Error = Wrap; // TODO crate::Error ?
        fn github_markdown_report<W: std::io::Write>(
            &self,
            cmd: &Relentless,
            w: &mut ReportWriter<W>,
        ) -> Result<(), Self::Error> {
            let Testcase { description, target, setting, .. } = self.testcases.coalesce();

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
                        writeln!(
                            w,
                            "<summary> {} message was found </summary>",
                            CaseGithubMarkdownReport::MESSAGE_EMOJI
                        )?;
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
}
