use std::{fmt::Debug, fs::File, path::Path};

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
    B: Body + Debug,
{
    type Error = std::io::Error;
    fn record<W: std::io::Write>(&self, w: &mut W) -> Result<(), Self::Error> {
        let (method, uri, version) = (self.method(), self.uri(), self.version());
        let headers = self.headers();
        let body = self.body();
        writeln!(w, "{} {} {:?}", method, uri, version)?;
        for (header, value) in headers.iter() {
            writeln!(w, "{}: {:?}", header, value)?;
        }
        writeln!(w)?;
        write!(w, "{:?}", body)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use bytes::Bytes;

    use super::*;

    #[test]
    fn test_record() {
        let request = http::Request::builder()
            .method("GET")
            .uri("http://localhost:3000")
            .body(http_body_util::Empty::<Bytes>::new())
            .unwrap();

        let mut buf = Vec::new();
        request.record(&mut buf).unwrap();
        // println!("{}", String::from_utf8_lossy(&buf));
        assert_eq!(buf, b"GET http://localhost:3000 HTTP/1.1\n\n");
    }
}
