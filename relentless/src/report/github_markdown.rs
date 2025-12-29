use crate::{
    report::{ReportWriter, Reporter},
    shot::job::JobReport,
};

pub struct GithubMarkdown;
impl<C, Q, P, M> Reporter<&JobReport<'_, C, Q, P, M>> for GithubMarkdown {
    type Error = std::fmt::Error;
    fn write_report<W: std::io::Write>(
        &self,
        writer: &mut ReportWriter<W>,
        report: &JobReport<C, Q, P, M>,
    ) -> Result<(), Self::Error> {
        todo!()
    }
}
