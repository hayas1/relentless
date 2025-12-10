use crate::{
    report::{ReportWriter, Reporter},
    shot::job::JobReport,
};

pub struct GithubMarkdown;
impl<C, Q, P> Reporter<&JobReport<'_, C, Q, P>> for GithubMarkdown {
    type Error = std::fmt::Error;
    fn write_report<W: std::io::Write>(
        &self,
        writer: &mut ReportWriter<W>,
        report: &JobReport<C, Q, P>,
    ) -> Result<(), Self::Error> {
        todo!()
    }
}
