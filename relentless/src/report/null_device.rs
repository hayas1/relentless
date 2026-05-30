use crate::{
    report::{ReportWriter, Reporter},
    shot::job::JobReport,
};

pub struct NullDevice;
impl<C, Q, P, M> Reporter<&JobReport<'_, C, Q, P, M>> for NullDevice {
    type Error = std::fmt::Error;
    fn write_report<W: std::io::Write>(
        &self,
        _: &mut ReportWriter<W>,
        _: &JobReport<C, Q, P, M>,
    ) -> Result<(), Self::Error> {
        Ok(())
    }
}
