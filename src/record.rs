use std::{fmt::Display, fs::File, path::Path};

use http::request::Parts;
use http_body::Body;

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

impl<B> Recordable for http::Request<B>
where
    Self: Clone,
    B: Body + Clone + Display,
{
    type Error = std::io::Error;
    fn record<W: std::io::Write>(&self, w: &mut W) -> Result<(), Self::Error> {
        let (Parts { method, uri, version, headers, .. }, body) = self.clone().into_parts();
        writeln!(w, "{} {} {:?}", method, uri, version)?;
        for (header, value) in headers.iter() {
            writeln!(w, "{}: {:?}", header, value)?;
        }
        writeln!(w, "{}", body)?;
        Ok(())
    }
}
