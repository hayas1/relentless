use std::{
    collections::HashMap,
    fs::{read_to_string, File},
    path::Path,
    time::Duration,
};

use http::{HeaderMap, Method};
use serde::{Deserialize, Serialize};

use crate::error::{FormatError, RelentlessResult};

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
    pub destinations: HashMap<String, String>, // TODO HashMap<String, Uri>
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
    pub repeat: Option<usize>,
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
}

impl Config {
    pub fn read<P: AsRef<Path>>(path: P) -> RelentlessResult<Self> {
        Ok(Format::from_path(path.as_ref())?.deserialize_testcase(path.as_ref())?)
    }
    pub fn read_str(s: &str, format: Format) -> RelentlessResult<Self> {
        Ok(format.deserialize_testcase_str(s)?)
    }
}
impl Setting {
    pub fn coalesce(&self, other: &Self) -> Self {
        Self {
            protocol: self.protocol.clone().or(other.protocol.clone()),
            template: if self.template.is_empty() { other.template.clone() } else { self.template.clone() },
            repeat: self.repeat.or(other.repeat),
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
