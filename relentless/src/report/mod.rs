use std::io::BufWriter;

use clap::ValueEnum;
use serde::{Deserialize, Serialize};

pub mod console;
pub mod github_markdown;
pub mod null_device;

#[cfg_attr(feature = "cli", derive(ValueEnum))]
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

pub trait Report<R> {
    type Error;
    fn report(&self, report: R) -> Result<(), Self::Error> {
        let mut writer = ReportWriter::new(0, BufWriter::new(std::io::stdout()));
        self.write_report(&mut writer, report)
    }
    fn write_report<W: std::io::Write>(&self, writer: &mut ReportWriter<W>, report: R) -> Result<(), Self::Error>;
}
impl<R, E> Report<R> for ReportFormat
where
    null_device::NullDeviceReport: Report<R, Error = E>,
    console::ConsoleReport: Report<R, Error = E>,
    github_markdown::GithubMarkdownReport: Report<R, Error = E>,
{
    type Error = E;
    fn write_report<W: std::io::Write>(&self, writer: &mut ReportWriter<W>, report: R) -> Result<(), Self::Error> {
        match self {
            ReportFormat::NullDevice => null_device::NullDeviceReport.write_report(writer, report),
            ReportFormat::Console => console::ConsoleReport.write_report(writer, report),
            ReportFormat::GithubMarkdown => github_markdown::GithubMarkdownReport.write_report(writer, report),
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
