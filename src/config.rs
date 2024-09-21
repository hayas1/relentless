use std::{
    collections::HashMap,
    fs::{read_to_string, File},
    path::Path,
    time::Duration,
};

use http::{HeaderMap, Method};
use hyper::body::Body;
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
    pub setting: Setting,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(deny_unknown_fields)]
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
    pub body: Option<String>,
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

    pub fn instance<S, Req, Res>(
        self,
        clients: Option<HashMap<String, S>>,
    ) -> RelentlessResult<(Worker, Vec<CaseService<S, Req, Res>>)>
    where
        Req: Clone + Body + Send + Sync + 'static,
        Req::Data: Send + 'static,
        Req::Error: std::error::Error + Sync + Send + 'static,
        Res: Send + Sync + 'static,
        S: Clone + Service<http::Request<Req>, Response = http::Response<Res>> + Send + Sync + 'static,
        S::Future: 'static,
        S::Error: Send + 'static,
        RelentlessError: From<S::Error>,
    {
        let Self { worker_config, testcase } = self;

        let worker = Self::worker(worker_config)?;
        let cases = testcase.into_iter().map(|tc| Self::case(tc, clients.clone())).collect::<Result<Vec<_>, _>>()?;
        Ok((worker, cases))
    }

    pub fn worker(config: WorkerConfig) -> RelentlessResult<Worker> {
        // TODO layer
        Ok(Worker::new(config))
    }

    pub fn case<S, Req, Res>(
        testcase: Testcase,
        clients: Option<HashMap<String, S>>,
    ) -> RelentlessResult<CaseService<S, Req, Res>>
    where
        Req: Clone + Body + Send + Sync + 'static,
        Req::Data: Send + 'static,
        Req::Error: std::error::Error + Sync + Send + 'static,
        Res: Send + Sync + 'static,
        S: Clone + Service<http::Request<Req>, Response = http::Response<Res>> + Send + Sync + 'static,
        S::Future: 'static,
        S::Error: Send + 'static,
        RelentlessError: From<S::Error>,
    {
        // TODO layer
        // TODO!!! coalesce protocol
        let protocol = &testcase.setting.protocol;
        match (protocol, clients) {
            (&None, None) => Ok(CaseService::Http(Case::new_http(testcase))),
            (&None, Some(c)) => Ok(CaseService::Default(Case::new(testcase, c))),
            (&Some(Protocol::Http(_)), _) => Ok(CaseService::Http(Case::new_http(testcase))),
        }
    }
}
impl Setting {
    pub fn coalesce(&self, other: &Self) -> Self {
        Self {
            protocol: self.protocol.clone().or(other.protocol.clone()),
            origin: if self.origin.is_empty() { other.origin.clone() } else { self.origin.clone() },
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
