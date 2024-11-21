use std::{fs::File, path::Path};

pub trait Recordable {
    type Error;
    fn record<W: std::io::Write>(&self, w: &mut W) -> Result<(), Self::Error>;
    fn record_file(&self, file: &mut File) -> Result<(), Self::Error> {
        self.record(file)
    }
    fn record_path<P>(&self, path: P) -> Result<(), Self::Error>
    where
        P: AsRef<Path>,
        Self::Error: From<std::io::Error>,
    {
        self.record_file(&mut File::create(path.as_ref())?)
    }
}
