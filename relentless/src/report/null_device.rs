use crate::{
    report::{Report, ReportWriter},
    shot::job::JobReport,
};

pub struct NullDeviceReport;
impl<Q, P> Report<&JobReport<'_, Q, P>> for NullDeviceReport {
    type Error = std::fmt::Error;
    fn write_report<W: std::io::Write>(&self, _: &mut ReportWriter<W>, _: &JobReport<Q, P>) -> Result<(), Self::Error> {
        Ok(())
    }
}
