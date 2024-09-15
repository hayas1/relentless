use std::{
    collections::HashMap,
    fs::{read_to_string, File},
    path::Path,
    time::Duration,
};

use http::{HeaderMap, Method};
use reqwest::Request;
use serde::{Deserialize, Serialize};
use tower::timeout::TimeoutLayer;

use crate::{
    error::{FormatError, HttpError, RelentlessResult},
    worker::{Case, Worker},
};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct Config {
    pub name: Option<String>,
    pub setting: Option<Setting>,
    pub testcase: Vec<Testcase>,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct Setting {
    #[serde(flatten)]
    pub protocol: Option<Protocol>,
    #[serde(default)]
    pub origin: HashMap<String, String>,
    #[serde(default)]
    pub template: HashMap<String, HashMap<String, String>>,
    #[serde(default)]
    pub timeout: Option<Duration>,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Protocol {
    Http(Http),
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct Http {
    #[serde(default, with = "http_serde::option::method")]
    pub method: Option<Method>,
    #[serde(default, with = "http_serde::option::header_map")]
    pub header: Option<HeaderMap>,
    #[serde(default)]
    pub body: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct Testcase {
    pub description: Option<String>,
    pub target: String,
    pub setting: Option<Setting>,
    pub attr: Option<Attribute>,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct Attribute {
    #[serde(default)]
    pub allow: bool,
    #[serde(default)]
    pub repeat: Option<usize>,
}

impl Config {
    pub fn read<P: AsRef<Path>>(path: P) -> RelentlessResult<Self> {
        Ok(Format::from_path(path.as_ref())?.deserialize_testcase(path.as_ref())?)
    }
    pub fn read_str(s: &str, format: Format) -> RelentlessResult<Self> {
        Ok(format.deserialize_testcase_str(s)?)
    }
    pub fn read_dir<P: AsRef<Path>>(path: P) -> RelentlessResult<Vec<Self>> {
        // TODO logging read files
        // TODO filter by format
        std::fs::read_dir(path)?.map(|f| Self::read(f?.path())).filter(Result::is_ok).collect::<Result<Vec<_>, _>>()
    }

    pub fn instance(self) -> RelentlessResult<(Worker<TimeoutLayer>, Vec<Case<TimeoutLayer>>)> {
        let Self { name, setting, testcase } = self;

        let worker = Self::worker(name, setting)?;
        let cases = testcase.into_iter().map(Self::case).collect::<Result<Vec<_>, _>>()?;
        Ok((worker, cases))
    }

    pub fn worker(name: Option<String>, setting: Option<Setting>) -> RelentlessResult<Worker<TimeoutLayer>> {
        Ok(Worker::new(name, setting.unwrap_or_default(), None))
    }

    pub fn case(testcase: Testcase) -> RelentlessResult<Case<TimeoutLayer>> {
        let Testcase { description, target, setting, attr } = testcase;

        Ok(Case::new(description, target, setting.unwrap_or_default(), attr.unwrap_or_default(), None))
    }
}
impl Setting {
    pub fn coalesce(self, other: Self) -> Self {
        Self {
            protocol: self.protocol.or(other.protocol),
            origin: if self.origin.is_empty() { other.origin } else { self.origin },
            template: if self.template.is_empty() { other.template } else { self.template },
            timeout: self.timeout.or(other.timeout),
        }
    }

    pub fn requests(self, target: &str) -> RelentlessResult<HashMap<String, Request>> {
        let Self { protocol, origin, template, timeout } = self;
        Ok(origin
            .into_iter()
            .map(|(name, origin)| {
                let (method, headers, body) = match protocol.clone() {
                    Some(Protocol::Http(http)) => (http.method, http.header, http.body),
                    None => (None, None, None),
                };
                let url = reqwest::Url::parse(&origin)?.join(target)?;
                let mut request = Request::new(method.unwrap_or(Method::GET), url);
                *request.timeout_mut() = timeout.or(Some(Duration::from_secs(10)));
                *request.headers_mut() = headers.unwrap_or_default();
                *request.body_mut() = body.map(|b| b.into());
                Ok::<_, HttpError>((name, request))
            })
            .collect::<Result<HashMap<_, _>, _>>()?)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Format {
    #[cfg(feature = "json")]
    Json,
    #[cfg(feature = "yaml")]
    Yaml,
    #[cfg(feature = "toml")]
    Toml,
}
impl Format {
    pub fn from_path<P: AsRef<Path>>(path: P) -> Result<Self, FormatError> {
        let basename = path.as_ref().extension().and_then(|ext| ext.to_str());
        match basename {
            #[cfg(feature = "json")]
            Some("json") => Ok(Format::Json),
            #[cfg(feature = "yaml")]
            Some("yaml" | "yml") => Ok(Format::Yaml),
            #[cfg(feature = "toml")]
            Some("toml") => Ok(Format::Toml),
            Some(ext) => Err(FormatError::UnknownFormatExtension(ext.to_string())),
            _ => Err(FormatError::CannotSpecifyFormat),
        }
    }

    pub fn deserialize_testcase<P: AsRef<Path>>(&self, path: P) -> Result<Config, FormatError> {
        match self {
            #[cfg(feature = "json")]
            Format::Json => Ok(serde_json::from_reader(File::open(path)?)?),
            #[cfg(feature = "yaml")]
            Format::Yaml => Ok(serde_yaml::from_reader(File::open(path)?)?),
            #[cfg(feature = "toml")]
            Format::Toml => Ok(toml::from_str(&read_to_string(path)?)?),
        }
    }

    pub fn deserialize_testcase_str(&self, content: &str) -> Result<Config, FormatError> {
        match self {
            #[cfg(feature = "json")]
            Format::Json => Ok(serde_json::from_str(content)?),
            #[cfg(feature = "yaml")]
            Format::Yaml => Ok(serde_yaml::from_str(content)?),
            #[cfg(feature = "toml")]
            Format::Toml => Ok(toml::from_str(content)?),
        }
    }
}
