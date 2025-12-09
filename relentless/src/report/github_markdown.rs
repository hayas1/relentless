use crate::{
    report::{Report, ReportWriter},
    shot::job::JobReport,
};

pub struct GithubMarkdownReport;
impl<Q, P> Report<&JobReport<'_, Q, P>> for GithubMarkdownReport {
    type Error = std::fmt::Error;
    fn write_report<W: std::io::Write>(
        &self,
        writer: &mut ReportWriter<W>,
        report: &JobReport<Q, P>,
    ) -> Result<(), Self::Error> {
        Ok(())
    }
}
