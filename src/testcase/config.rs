use std::{
    collections::HashMap,
    fs::{read_to_string, File},
    path::Path,
    time::Duration,
};

use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Config {
    pub name: Option<String>,
    pub setting: Setting,
    pub testcase: Vec<Http>,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Setting {
    pub origin: HashMap<String, String>,
    #[serde(default)]
    pub header: HashMap<String, Vec<String>>, // TODO use multi map ?
    #[serde(default)]
    pub template: HashMap<String, HashMap<String, String>>,
    #[serde(default = "default_timeout")]
    pub timeout: Duration,
}
// TODO Setting::default_timeout
pub fn default_timeout() -> Duration {
    Duration::from_secs(10)
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Http {
    pub description: Option<String>,
    pub method: String,
    pub pathname: String,
    pub setting: Option<Setting>,
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

#[derive(Error, Debug)]
pub enum FormatError {
    #[error("unknown format extension: {0}")]
    UnknownFormatExtension(String),
    #[error("cannot specify format")]
    CannotSpecifyFormat,

    #[error(transparent)]
    StdIoError(#[from] std::io::Error),
    #[cfg(feature = "json")]
    #[error(transparent)]
    JsonError(#[from] serde_json::Error),
    #[cfg(feature = "yaml")]
    #[error(transparent)]
    YamlError(#[from] serde_yaml::Error),
    #[cfg(feature = "toml")]
    #[error(transparent)]
    TomlError(#[from] toml::de::Error),
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
