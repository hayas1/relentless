use std::fmt::Write as _;

use crate::{
    report::{Report, ReportWriter},
    shot::{job::JobReport, suite::SuiteReport, testcase::CaseReport},
};

pub struct ConsoleReport;
impl<Q, P> Report<&JobReport<'_, Q, P>> for ConsoleReport {
    type Error = std::fmt::Error;
    fn write_report<W: std::io::Write>(
        &self,
        writer: &mut ReportWriter<W>,
        report: &JobReport<Q, P>,
    ) -> Result<(), Self::Error> {
        writeln!(writer, "{}", report.pass())?;
        Ok(())
    }
}
impl<Q, P> Report<&SuiteReport<'_, Q, P>> for ConsoleReport {
    type Error = std::fmt::Error;
    fn write_report<W: std::io::Write>(
        &self,
        writer: &mut ReportWriter<W>,
        report: &SuiteReport<Q, P>,
    ) -> Result<(), Self::Error> {
        Ok(())
    }
}
impl<Q, P> Report<&CaseReport<'_, Q, P>> for ConsoleReport {
    type Error = std::fmt::Error;
    fn write_report<W: std::io::Write>(
        &self,
        writer: &mut ReportWriter<W>,
        report: &CaseReport<Q, P>,
    ) -> Result<(), Self::Error> {
        Ok(())
    }
}
