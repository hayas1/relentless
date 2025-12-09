use crate::{
    report::{ReportWriter, Reporter},
    shot::job::JobReport,
};

pub struct GithubMarkdown;
impl<Q, P> Reporter<&JobReport<'_, Q, P>> for GithubMarkdown {
    type Error = std::fmt::Error;
    fn write_report<W: std::io::Write>(
        &self,
        writer: &mut ReportWriter<W>,
        report: &JobReport<Q, P>,
    ) -> Result<(), Self::Error> {
        Ok(())
    }
}
