use crate::{
    report::{ReportSpec, ReportWriter, Reporter},
    shot::job::JobReport,
};

pub struct GithubMarkdown<'a> {
    pub spec: &'a ReportSpec,
}
impl<'a> GithubMarkdown<'a> {
    pub fn new(spec: &'a ReportSpec) -> Self {
        Self { spec }
    }
}
impl<C, Q, P, M> Reporter<&JobReport<'_, C, Q, P, M>> for GithubMarkdown<'_> {
    type Error = std::fmt::Error;
    fn write_report<W: std::io::Write>(
        &self,
        writer: &mut ReportWriter<W>,
        report: &JobReport<C, Q, P, M>,
    ) -> Result<(), Self::Error> {
        todo!()
    }
}
