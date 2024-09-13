use std::{
    collections::HashMap,
    fs::{read_to_string, File},
    path::Path,
    str::FromStr,
    time::Duration,
};

use reqwest::Request;
use serde::{Deserialize, Serialize};
use tower::timeout::TimeoutLayer;

use crate::{
    error::{FormatError, HttpError, RelentlessResult},
    worker::{Unit, Worker},
};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Config {
    pub name: Option<String>,
    pub setting: Option<Setting>,
    pub testcase: Vec<Testcase>,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Setting {
    pub origin: HashMap<String, String>,
    #[serde(default)]
    pub header: HashMap<String, Vec<String>>, // TODO use multi map ?
    #[serde(default)]
    pub template: HashMap<String, HashMap<String, String>>,
    #[serde(default = "Config::default_timeout")]
    pub timeout: Duration,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Testcase {
    pub description: Option<String>,
    pub method: String,
    pub pathname: String,
    pub setting: Option<Setting>,
}

impl Config {
    pub fn read<P: AsRef<Path>>(path: P) -> RelentlessResult<Self> {
        Ok(Format::from_path(path.as_ref())?.import_testcase(path.as_ref())?)
    }

    pub fn instance(self) -> RelentlessResult<(Worker<TimeoutLayer>, Vec<Unit<TimeoutLayer>>)> {
        let worker = self.worker()?;
        let units = self
            .testcase
            .iter()
            .map(|t| self.unit(t))
            .collect::<Result<Vec<_>, _>>()?;
        Ok((worker, units))
    }

    pub fn worker(&self) -> RelentlessResult<Worker<TimeoutLayer>> {
        let timeout = self.setting.clone().unwrap().timeout;
        Ok(Worker::new(
            self.name.clone(),
            Some(TimeoutLayer::new(timeout)),
            self.setting.clone(),
        ))
    }

    pub fn unit(&self, testcase: &Testcase) -> RelentlessResult<Unit<TimeoutLayer>> {
        let description = testcase.description.clone();
        let requests = Self::to_requests(&self.setting.clone().unwrap(), testcase)?;

        Ok(Unit::new(
            description,
            requests,
            None,
            testcase.setting.clone(),
        ))
    }

    pub fn to_requests(setting: &Setting, testcase: &Testcase) -> RelentlessResult<Vec<Request>> {
        Ok(setting
            .origin
            .values()
            .map(|origin| {
                let method = reqwest::Method::from_str(&testcase.method)?;
                let url = reqwest::Url::parse(origin)?.join(&testcase.pathname)?;
                Ok::<_, HttpError>(Request::new(method, url))
            })
            .collect::<Result<Vec<_>, _>>()?)
    }

    pub fn default_timeout() -> Duration {
        Duration::from_secs(10)
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

    pub fn import_testcase<P: AsRef<Path>>(&self, path: P) -> Result<Config, FormatError> {
        match self {
            #[cfg(feature = "json")]
            Format::Json => Ok(serde_json::from_reader(File::open(path)?)?),
            #[cfg(feature = "yaml")]
            Format::Yaml => Ok(serde_yaml::from_reader(File::open(path)?)?),
            #[cfg(feature = "toml")]
            Format::Toml => Ok(toml::from_str(&read_to_string(path)?)?),
        }
    }

    pub fn import_testcase_str(&self, content: &str) -> Result<Config, FormatError> {
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
