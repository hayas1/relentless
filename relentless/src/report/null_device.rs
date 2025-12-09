use crate::{
    report::{ReportWriter, Reporter},
    shot::job::JobReport,
};

pub struct NullDevice;
impl<Q, P> Reporter<&JobReport<'_, Q, P>> for NullDevice {
    type Error = std::fmt::Error;
    fn write_report<W: std::io::Write>(&self, _: &mut ReportWriter<W>, _: &JobReport<Q, P>) -> Result<(), Self::Error> {
        Ok(())
    }
}
