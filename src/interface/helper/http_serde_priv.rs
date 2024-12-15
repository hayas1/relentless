use std::ops::{Deref, DerefMut};

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Default, Hash, Serialize, Deserialize)]
pub struct Method(#[serde(with = "http_serde::method")] pub http::Method);
impl From<http::Method> for Method {
    fn from(m: http::Method) -> Self {
        Self(m)
    }
}
impl Deref for Method {
    type Target = http::Method;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}
impl DerefMut for Method {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Default, Hash, Serialize, Deserialize)]
pub struct StatusCode(#[serde(with = "http_serde::status_code")] pub http::StatusCode);
impl From<http::StatusCode> for StatusCode {
    fn from(s: http::StatusCode) -> Self {
        Self(s)
    }
}
impl Deref for StatusCode {
    type Target = http::StatusCode;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}
impl DerefMut for StatusCode {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Uri(#[serde(with = "http_serde::uri")] pub http::Uri);
impl From<http::Uri> for Uri {
    fn from(u: http::Uri) -> Self {
        Self(u)
    }
}
impl Deref for Uri {
    type Target = http::Uri;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}
impl DerefMut for Uri {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Default, Hash, Serialize, Deserialize)]
pub struct Version(#[serde(with = "http_serde::version")] pub http::Version);
impl From<http::Version> for Version {
    fn from(v: http::Version) -> Self {
        Self(v)
    }
}
impl Deref for Version {
    type Target = http::Version;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}
impl DerefMut for Version {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct HeaderMap(#[serde(with = "http_serde::header_map")] pub http::HeaderMap);
impl From<http::HeaderMap> for HeaderMap {
    fn from(m: http::HeaderMap) -> Self {
        Self(m)
    }
}
impl Deref for HeaderMap {
    type Target = http::HeaderMap;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}
impl DerefMut for HeaderMap {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Authority(#[serde(with = "http_serde::authority")] pub http::uri::Authority);
impl From<http::uri::Authority> for Authority {
    fn from(a: http::uri::Authority) -> Self {
        Self(a)
    }
}
impl Deref for Authority {
    type Target = http::uri::Authority;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}
impl DerefMut for Authority {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}
