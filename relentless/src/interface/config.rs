use std::{
    collections::HashMap,
    fmt::Debug,
    fs::{read_to_string, File},
    path::Path,
    time::Duration,
};

use serde::{de::DeserializeOwned, Deserialize, Serialize};

use crate::{
    assault::destinations::Destinations,
    error::{InterfaceError, IntoResult},
    interface::template::Template,
};

use super::helper::{coalesce::Coalesce, http_serde_priv, is_default::IsDefault, transpose};

// TODO this trait should be divided
pub trait Configuration: Debug + Clone + PartialEq + Eq + Serialize + DeserializeOwned + Default {}
impl<T> Configuration for T where T: Debug + Clone + PartialEq + Eq + Serialize + DeserializeOwned + Default {}

#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "kebab-case")]
#[serde(bound = "Q: Configuration, P: Configuration")]
pub struct Config<Q, P> {
    #[serde(flatten, default, skip_serializing_if = "IsDefault::is_default")]
    pub worker_config: WorkerConfig<Q, P>,

    #[serde(default, skip_serializing_if = "IsDefault::is_default")]
    pub testcases: Vec<Testcase<Q, P>>,
}
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "kebab-case")]
#[serde(bound = "Q: Configuration, P: Configuration")]
pub struct WorkerConfig<Q, P> {
    #[serde(default, skip_serializing_if = "IsDefault::is_default")]
    pub name: Option<String>,
    #[serde(default, skip_serializing_if = "IsDefault::is_default")]
    pub destinations: Destinations<http_serde_priv::Uri>,
    #[serde(default, skip_serializing_if = "IsDefault::is_default")]
    pub setting: Setting<Q, P>,
}
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "kebab-case")]
#[serde(bound = "Q: Configuration, P: Configuration")]
pub struct Setting<Q, P> {
    #[serde(default, skip_serializing_if = "IsDefault::is_default")]
    pub request: Q,

    #[serde(default, skip_serializing_if = "IsDefault::is_default", with = "transpose::transpose_template_serde")]
    pub template: Destinations<Template>,
    #[serde(default, skip_serializing_if = "IsDefault::is_default")]
    pub repeat: Repeat,
    #[serde(default, skip_serializing_if = "IsDefault::is_default")]
    pub timeout: Option<Duration>, // TODO parse from string? https://crates.io/crates/humantime ?
    #[serde(default, skip_serializing_if = "IsDefault::is_default")]
    pub allow: Option<bool>,

    #[serde(default, skip_serializing_if = "IsDefault::is_default")]
    pub response: P,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Default, Hash, Serialize, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "kebab-case")]
pub struct Repeat(pub Option<usize>);
impl Coalesce for Repeat {
    fn coalesce(self, other: &Self) -> Self {
        Self(self.0.or(other.0))
    }
}
impl Repeat {
    pub fn range(&self) -> std::ops::Range<usize> {
        0..self.times()
    }
    pub fn times(&self) -> usize {
        self.0.unwrap_or(1)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "kebab-case")]
pub enum Severity {
    Allow,
    Warn,
    Deny,
}

#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "kebab-case")]
#[serde(bound = "Q: Configuration, P: Configuration")]
pub struct Testcase<Q, P> {
    #[serde(default, skip_serializing_if = "IsDefault::is_default")]
    pub description: Option<String>,
    pub target: String,

    #[serde(default, skip_serializing_if = "IsDefault::is_default")]
    pub setting: Setting<Q, P>,
}

impl<Q: Configuration, P: Configuration> Config<Q, P> {
    pub fn read<A: AsRef<Path>>(path: A) -> crate::Result<Self> {
        Ok(Format::from_path(path.as_ref())?
            .deserialize_testcase(path.as_ref())
            .map_err(|e| InterfaceError::CannotReadConfig(path.as_ref().display().to_string(), e))?)
    }
    pub fn read_str(s: &str, format: Format) -> crate::Result<Self> {
        format.deserialize_testcase_str(s)
    }
}
impl<Q: Coalesce, P: Coalesce> Coalesce<Destinations<http_serde_priv::Uri>> for WorkerConfig<Q, P> {
    fn coalesce(self, other: &Destinations<http_serde_priv::Uri>) -> Self {
        let destinations = self.destinations.coalesce(&other.iter().map(|(k, v)| (k.to_string(), v.clone())).collect());
        Self { destinations, ..self }
    }
}
impl<T: Clone> Coalesce<HashMap<String, T>> for Destinations<T> {
    fn coalesce(self, other: &HashMap<String, T>) -> Self {
        // TODO Coalesce trait should be renamed because override usage may be inverse of coalesce
        let mut destinations = self.clone();
        for (name, dest) in other {
            destinations.entry(name.to_string()).and_modify(|d| *d = dest.clone());
        }
        destinations
    }
}

impl<Q: Coalesce, P: Coalesce> Coalesce<Setting<Q, P>> for Testcase<Q, P> {
    fn coalesce(self, other: &Setting<Q, P>) -> Self {
        let setting = self.setting.coalesce(other);
        Self { setting, ..self }
    }
}
impl<Q: Coalesce, P: Coalesce> Coalesce for Setting<Q, P> {
    fn coalesce(self, other: &Self) -> Self {
        Self {
            request: self.request.coalesce(&other.request),
            template: if self.template.is_empty() { other.template.clone() } else { self.template },
            repeat: self.repeat.coalesce(&other.repeat),
            timeout: self.timeout.or(other.timeout),
            allow: self.allow.or(other.allow),
            response: self.response.coalesce(&other.response),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Format {
    #[cfg(feature = "json")]
    Json,
    #[cfg(feature = "yaml")]
    Yaml,
    #[cfg(feature = "toml")]
    Toml,
}
impl Format {
    pub fn from_path<A: AsRef<Path>>(path: A) -> crate::Result<Self> {
        let basename = path.as_ref().extension().and_then(|ext| ext.to_str());
        match basename {
            #[cfg(feature = "json")]
            Some("json") => Ok(Format::Json),
            #[cfg(feature = "yaml")]
            Some("yaml" | "yml") => Ok(Format::Yaml),
            #[cfg(feature = "toml")]
            Some("toml") => Ok(Format::Toml),
            Some(ext) => Err(InterfaceError::UnknownFormatExtension(ext.to_string()))?,
            _ => Err(InterfaceError::CannotSpecifyFormat)?,
        }
    }

    pub fn deserialize_testcase<A: AsRef<Path>, Q: Configuration, P: Configuration>(
        &self,
        path: A,
    ) -> crate::Result<Config<Q, P>> {
        match self {
            #[cfg(feature = "json")]
            Format::Json => Ok(serde_json::from_reader(File::open(path).box_err()?).box_err()?),
            #[cfg(feature = "yaml")]
            Format::Yaml => Ok(serde_yaml::from_reader(File::open(path).box_err()?).box_err()?),
            #[cfg(feature = "toml")]
            Format::Toml => Ok(toml::from_str(&read_to_string(path).box_err()?).box_err()?),
            #[cfg(not(any(feature = "json", feature = "yaml", feature = "toml")))]
            _ => Err(InterfaceError::UndefinedSerializeFormatPath(path.as_ref().display().to_string()))?,
        }
    }

    pub fn deserialize_testcase_str<Q: Configuration, P: Configuration>(
        &self,
        content: &str,
    ) -> crate::Result<Config<Q, P>> {
        match self {
            #[cfg(feature = "json")]
            Format::Json => Ok(serde_json::from_str(content).box_err()?),
            #[cfg(feature = "yaml")]
            Format::Yaml => Ok(serde_yaml::from_str(content).box_err()?),
            #[cfg(feature = "toml")]
            Format::Toml => Ok(toml::from_str(content).box_err()?),
            #[cfg(not(any(feature = "json", feature = "yaml", feature = "toml")))]
            _ => Err(InterfaceError::UndefinedSerializeFormatContent(content.to_string()))?,
        }
    }
}

#[cfg(test)]
mod tests {
    // use super::*;

    // #[test]
    // #[cfg(not(any(feature = "json", feature = "yaml", feature = "toml")))]
    // fn test_no_default_features() {
    //     let err = Config::<(HttpRequest), HttpResponse>::read("path/to/config.yaml").unwrap_err();
    //     assert!(matches!(err.downcast_ref().unwrap(), InterfaceError::UnknownFormatExtension(s) if s == "yaml"));
    // }
}
