use std::{
    collections::HashMap,
    fs::{read_to_string, File},
    path::Path,
    time::Duration,
};

use bytes::Bytes;
use http::{HeaderMap, Method};
use http_body_util::{combinators::UnsyncBoxBody, Empty};
use hyper::body::{Body, Incoming};
use serde::{Deserialize, Serialize};
use tower::Service;

use crate::{
    error::{FormatError, RelentlessError, RelentlessResult},
    worker::{Case, CaseService, Worker},
};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(deny_unknown_fields)]
pub struct Config {
    #[serde(flatten, default)]
    pub worker_config: WorkerConfig,

    pub testcase: Vec<Testcase>,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(deny_unknown_fields)]
pub struct WorkerConfig {
    pub name: Option<String>,
    #[serde(default)]
    pub origins: HashMap<String, String>,
    #[serde(default)]
    pub setting: Setting,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(deny_unknown_fields)]
pub struct Setting {
    #[serde(flatten)]
    pub protocol: Option<Protocol>,
    #[serde(default)]
    pub template: HashMap<String, HashMap<String, String>>,
    #[serde(default)]
    pub timeout: Option<Duration>,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "snake_case")]
pub enum Protocol {
    Http(Http),
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(deny_unknown_fields)]
pub struct Http {
    #[serde(default, with = "http_serde::option::method")]
    pub method: Option<Method>,
    #[serde(default, with = "http_serde::option::header_map")]
    pub header: Option<HeaderMap>,
    #[serde(default)]
    pub body: Option<BodyStructure>,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub enum BodyStructure {
    Empty,
}
impl Default for BodyStructure {
    fn default() -> Self {
        Self::Empty
    }
}
pub trait FromBodyStructure {
    fn from_body_structure(val: BodyStructure) -> Self;
}
impl<T> FromBodyStructure for T
where
    T: Body + Default, // TODO other than Default
{
    fn from_body_structure(body: BodyStructure) -> Self {
        match body {
            BodyStructure::Empty => Default::default(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(deny_unknown_fields)]
pub struct Testcase {
    pub description: Option<String>,
    pub target: String,

    #[serde(default)]
    pub setting: Setting,
    #[serde(default)]
    pub attr: Attribute,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(deny_unknown_fields)]
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

    pub fn instance<S, ReqB, ResB>(
        self,
        // clients: Option<HashMap<String, S>>,
    ) -> RelentlessResult<Vec<CaseService<S, ReqB, ResB>>>
    where
        ReqB: Body + FromBodyStructure + Send + 'static,
        ReqB::Data: Send + 'static,
        ReqB::Error: std::error::Error + Sync + Send + 'static,
        ResB: From<Bytes> + Send + 'static,
        S: Clone + Service<http::Request<ReqB>, Response = http::Response<ResB>> + Send + Sync + 'static,
        RelentlessError: From<S::Error>,
    {
        let Self { worker_config, testcase } = self;

        // let worker = Self::worker(worker_config, clients)?;
        let cases = testcase.into_iter().map(Self::case).collect::<Result<Vec<_>, _>>()?;
        Ok(cases)
    }

    // pub fn worker<S, ReqB, ResB>(
    //     config: WorkerConfig,
    //     clients: Option<HashMap<String, S>>,
    // ) -> RelentlessResult<Worker<S, ReqB, ResB>>
    // where
    //     ReqB: Body + FromBodyStructure + Send + 'static,
    //     ReqB::Data: Send + 'static,
    //     ReqB::Error: std::error::Error + Sync + Send + 'static,
    //     ResB: From<Bytes> + Send + 'static,
    //     S: Clone + Service<http::Request<ReqB>, Response = http::Response<ResB>> + Send + Sync + 'static,
    //     RelentlessError: From<S::Error>,
    // {
    //     // TODO layer
    //     Worker::new(config, clients)
    // }

    pub fn case<S, ReqB, ResB>(testcase: Testcase) -> RelentlessResult<CaseService<S, ReqB, ResB>>
    where
        ReqB: Body + FromBodyStructure + Send + 'static,
        ReqB::Data: Send + 'static,
        ReqB::Error: std::error::Error + Sync + Send + 'static,
        ResB: From<Bytes> + Send + 'static,
        S: Clone + Service<http::Request<ReqB>, Response = http::Response<ResB>> + Send + Sync + 'static,
        RelentlessError: From<S::Error>,
    {
        // TODO layer
        // TODO!!! coalesce protocol
        let protocol = &testcase.setting.protocol;
        match protocol {
            None => Ok(CaseService::Http(Case::new_http(testcase))),
            Some(Protocol::Http(_)) => Ok(CaseService::Http(Case::new_http(testcase))),
        }
    }
}
impl Setting {
    pub fn coalesce(&self, other: &Self) -> Self {
        Self {
            protocol: self.protocol.clone().or(other.protocol.clone()),
            template: if self.template.is_empty() { other.template.clone() } else { self.template.clone() },
            timeout: self.timeout.or(other.timeout),
        }
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
