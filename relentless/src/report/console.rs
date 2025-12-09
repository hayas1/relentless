use std::fmt::Write as _;

use crate::{
    report::{ReportWriter, Reporter},
    shot::{job::JobReport, suite::SuiteReport, testcase::CaseReport},
};

pub struct Console;
impl<Q, P> Reporter<&JobReport<'_, Q, P>> for Console {
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
impl<Q, P> Reporter<&SuiteReport<'_, Q, P>> for Console {
    type Error = std::fmt::Error;
    fn write_report<W: std::io::Write>(
        &self,
        writer: &mut ReportWriter<W>,
        report: &SuiteReport<Q, P>,
    ) -> Result<(), Self::Error> {
        Ok(())
    }
}
impl<Q, P> Reporter<&CaseReport<'_, Q, P>> for Console {
    type Error = std::fmt::Error;
    fn write_report<W: std::io::Write>(
        &self,
        writer: &mut ReportWriter<W>,
        report: &CaseReport<Q, P>,
    ) -> Result<(), Self::Error> {
        Ok(())
    }
}
