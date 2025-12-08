use std::io::Write;

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
    fn report<W: Write>(&self, writer: &mut W, report: R) -> Result<(), Self::Error>;
}
impl<R, E> Report<R> for ReportFormat
where
    null_device::NullDeviceReport: Report<R, Error = E>,
    console::ConsoleReport: Report<R, Error = E>,
    github_markdown::GithubMarkdownReport: Report<R, Error = E>,
{
    type Error = E;
    fn report<W: Write>(&self, writer: &mut W, report: R) -> Result<(), Self::Error> {
        match self {
            ReportFormat::NullDevice => null_device::NullDeviceReport.report(writer, report),
            ReportFormat::Console => console::ConsoleReport.report(writer, report),
            ReportFormat::GithubMarkdown => github_markdown::GithubMarkdownReport.report(writer, report),
        }
    }
}
