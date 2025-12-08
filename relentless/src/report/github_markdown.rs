use std::io::Write;

use crate::{report::Report, shot::job::JobReport};

pub struct GithubMarkdownReport;
impl<Q, P> Report<&JobReport<'_, Q, P>> for GithubMarkdownReport {
    type Error = std::io::Error;
    fn report<W: Write>(&self, writer: &mut W, report: &JobReport<Q, P>) -> Result<(), Self::Error> {
        Ok(())
    }
}
