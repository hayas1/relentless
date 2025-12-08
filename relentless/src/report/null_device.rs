use std::io::Write;

use crate::{report::Report, shot::job::JobReport};

pub struct NullDeviceReport;
impl<Q, P> Report<&JobReport<'_, Q, P>> for NullDeviceReport {
    type Error = std::io::Error;
    fn report<W: Write>(&self, _: &mut W, _: &JobReport<Q, P>) -> Result<(), Self::Error> {
        Ok(())
    }
}
