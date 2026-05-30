use std::io::BufWriter;

use serde::{Deserialize, Serialize};

use crate::shot::job::JobSpec;

#[cfg(feature = "console-report")]
pub mod console;
pub mod github_markdown;
pub mod null_device;

#[cfg_attr(feature = "cli", derive(clap::Args))]
#[derive(Debug, Clone, PartialEq, Eq, Default, Hash, Serialize, Deserialize)]
pub struct ReportSpec {
    /// report only failed testcases
    #[cfg_attr(feature = "cli", arg(env, long))]
    pub ng_only: bool,

    /// without colorize output
    #[cfg_attr(feature = "cli", arg(env, long))]
    pub no_color: bool,
}

#[cfg_attr(feature = "cli", derive(clap::ValueEnum))]
#[derive(Debug, Clone, PartialEq, Eq, Default, Hash, Serialize, Deserialize)]
pub enum ReportFormat {
    /// without report
    #[cfg_attr(not(feature = "console-report"), default)]
    NullDevice,

    /// report to console
    #[cfg(feature = "console-report")]
    #[cfg_attr(feature = "console-report", default)]
    Console,

    /// report to markdown
    GithubMarkdown,
}

pub trait Reporter<R> {
    type Error;
    fn report(&self, report: R) -> Result<(), Self::Error> {
        let mut writer = ReportWriter::new(0, BufWriter::new(std::io::stdout()));
        self.write_report(&mut writer, report)
    }
    fn write_report<W: std::io::Write>(&self, writer: &mut ReportWriter<W>, report: R) -> Result<(), Self::Error>;
}
impl<R, E> Reporter<R> for JobSpec
where
    null_device::NullDevice: Reporter<R, Error = E>,
    for<'a> console::Console<'a>: Reporter<R, Error = E>,
    for<'a> github_markdown::GithubMarkdown<'a>: Reporter<R, Error = E>,
{
    type Error = E;
    fn write_report<W: std::io::Write>(&self, writer: &mut ReportWriter<W>, report: R) -> Result<(), Self::Error> {
        match self.report_format {
            ReportFormat::NullDevice => null_device::NullDevice.write_report(writer, report),
            ReportFormat::Console => console::Console::new(&self.report_spec).write_report(writer, report),
            ReportFormat::GithubMarkdown => {
                github_markdown::GithubMarkdown::new(&self.report_spec).write_report(writer, report)
            }
        }
    }
}

pub struct ReportWriter<W> {
    indent: usize,
    buf: W,
    at_start_line: bool,
}
impl<W> ReportWriter<W> {
    pub fn new(indent: usize, buf: W) -> Self {
        let at_start_line = true;
        Self { indent, buf, at_start_line }
    }
    pub fn indent(&self) -> String {
        " ".repeat(self.indent)
    }
    pub fn increment(&mut self, indent: usize) {
        self.indent += indent;
    }
    pub fn decrement(&mut self, indent: usize) {
        self.indent -= indent;
    }
    pub fn scope<F, T>(&mut self, f: F) -> T
    where
        F: FnOnce(&mut Self) -> T,
    {
        self.scope_n(2, f)
    }
    // TODO const generic, but F, T must be supplied explicitly like `self.scope_n::<1, _, _>(f)`
    pub fn scope_n<F, T>(&mut self, n: usize, f: F) -> T
    where
        F: FnOnce(&mut Self) -> T,
    {
        self.increment(n);
        let ret = f(self);
        self.decrement(n);
        ret
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
                writeln!(self.buf, "{line}").map_err(|_| std::fmt::Error)?;
                self.at_start_line = true;
            }
            self.at_start_line = s.ends_with('\n');
        } else {
            if self.at_start_line {
                write!(self.buf, "{}", self.indent()).map_err(|_| std::fmt::Error)?;
                self.at_start_line = false;
            }
            write!(self.buf, "{s}").map_err(|_| std::fmt::Error)?;
        }
        Ok(())
    }
}
