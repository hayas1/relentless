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

    #[serde(default)]
    pub testcase: Vec<Testcase>,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(deny_unknown_fields)]
pub struct WorkerConfig {
    pub name: Option<String>,
    #[serde(default)]
    pub destinations: Destinations<String>, // TODO Destination<Uri>, but serde_http doesn't support nested type other than Option
    #[serde(default)]
    pub setting: Setting,
}
pub type Destinations<T> = HashMap<String, T>;
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(deny_unknown_fields)]
pub struct Setting {
    #[serde(default, flatten)]
    pub protocol: Option<Protocol>,
    #[serde(default)]
    pub template: HashMap<String, Destinations<String>>,
    #[serde(default)]
    pub repeat: Option<usize>,
    #[serde(default)]
    pub timeout: Option<Duration>,
    #[serde(default, flatten)]
    pub evaluate: Option<Evaluate>,
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
#[serde(deny_unknown_fields, rename_all = "snake_case")]
pub enum BodyStructure {
    Empty,
}
impl Default for BodyStructure {
    fn default() -> Self {
        Self::Empty
    }
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "snake_case")]
pub enum Evaluate {
    PlainText(PlainTextEvaluate),
    #[cfg(feature = "json")]
    Json(JsonEvaluate),
}
impl Default for Evaluate {
    fn default() -> Self {
        Self::PlainText(PlainTextEvaluate {})
    }
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(deny_unknown_fields)]
pub struct PlainTextEvaluate {}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(deny_unknown_fields)]
#[cfg(feature = "json")]
pub struct JsonEvaluate {
    #[serde(default)]
    pub ignore: Vec<String>,
    #[serde(default)]
    pub patch: Option<PatchTo>,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "snake_case", untagged)]
#[cfg(feature = "json")]
pub enum PatchTo {
    All(json_patch::Patch),
    Destinations(Destinations<json_patch::Patch>),
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
impl Coalesce for WorkerConfig {
    type Other = Destinations<String>;
    fn coalesce(self, other: &Self::Other) -> Self {
        let destinations =
            self.destinations.coalesce(&other.iter().map(|(k, v)| (k.to_string(), v.to_string())).collect());
        Self { destinations, ..self }
    }
}
impl<T: Clone> Coalesce for Destinations<T> {
    type Other = Vec<(String, T)>;
    fn coalesce(self, other: &Self::Other) -> Self {
        // TODO Coalesce trait should be renamed because override usage may be inverse of coalesce
        let mut map = self.clone();
        for (name, dest) in other {
            map.entry(name.to_string()).and_modify(|d| *d = dest.clone());
        }
        map
    }
}

impl Coalesce for Testcase {
    type Other = Setting;
    fn coalesce(self, other: &Self::Other) -> Self {
        let setting = self.setting.coalesce(other);
        Self { setting, ..self }
    }
}
impl Coalesce for Setting {
    type Other = Self;
    fn coalesce(self, other: &Self) -> Self {
        Self {
            protocol: self.protocol.or(other.clone().protocol),
            template: if self.template.is_empty() { other.clone().template } else { self.template },
            repeat: self.repeat.or(other.repeat),
            timeout: self.timeout.or(other.timeout),
            evaluate: self.evaluate.or(other.clone().evaluate),
        }
    }
}

pub trait Coalesce {
    type Other;
    fn coalesce(self, other: &Self::Other) -> Self;
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct Coalesced<T, U> {
    base: T,
    coalesced: Vec<U>,
}
// TODO do not require S: Default
impl<T: Clone + Coalesce<Other = U>, U> Coalesced<T, U> {
    pub fn new(base: T, coalesced: Vec<U>) -> Self {
        Self { base, coalesced }
    }
    pub fn tuple(base: T, other: U) -> Self {
        Self::new(base, vec![other])
    }
    pub fn coalesce(&self) -> T {
        self.coalesced.iter().fold(self.base.clone(), |acc, x| acc.coalesce(x))
    }
    pub fn base(&self) -> &T {
        &self.base
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_read_example() {
        let _assault = Config::read("examples/config/assault.yaml");
        // TODO assert

        let _compare = Config::read("examples/config/compare.yaml");
        // TODO assert
    }

    #[test]
    fn test_config() {
        let example = Config {
            worker_config: WorkerConfig { name: Some("example".to_string()), ..Default::default() },
            testcase: vec![Testcase {
                description: Some("test description".to_string()),
                target: "/information".to_string(),
                setting: Setting {
                    evaluate: Some(Evaluate::Json(JsonEvaluate {
                        ignore: vec!["/datetime".to_string()],
                        // patch: Some(PatchTo::All(
                        //     serde_json::from_value(
                        //         serde_json::json!([{"op": "replace", "path": "/datetime", "value": "2021-01-01"}]),
                        //     )
                        //     .unwrap(),
                        // )),
                        patch: Some(PatchTo::Destinations(Destinations::from([
                            (
                                "actual".to_string(),
                                serde_json::from_value(serde_json::json!([{"op": "remove", "path": "/datetime"}]))
                                    .unwrap(),
                            ),
                            (
                                "expect".to_string(),
                                serde_json::from_value(serde_json::json!([{"op": "remove", "path": "/datetime"}]))
                                    .unwrap(),
                            ),
                        ]))),
                    })),
                    ..Default::default()
                },
                attr: Attribute { allow: true },
            }],
        };
        let yaml = serde_yaml::to_string(&example).unwrap();
        // println!("{}", yaml);

        let round_trip: Config = serde_yaml::from_str(&yaml).unwrap();
        assert_eq!(example, round_trip);
    }

    #[test]
    #[cfg(all(feature = "yaml", feature = "json"))]
    fn test_config_json_patch() {
        let all_yaml = r#"
        name: json patch to all
        destinations:
          actual: http://localhost:3000
          expect: http://localhost:3000
        testcase:
        - description: test description
          target: /information
          setting:
            json:
              patch:
              - op: replace
                path: /datetime
                value: 2021-01-01
        "#;
        let config = Config::read_str(all_yaml, Format::Yaml).unwrap();
        assert_eq!(
            config.testcase[0].setting.evaluate,
            Some(Evaluate::Json(JsonEvaluate {
                ignore: vec![],
                patch: Some(PatchTo::All(
                    serde_json::from_value(
                        serde_json::json!([{"op": "replace", "path": "/datetime", "value": "2021-01-01"}])
                    )
                    .unwrap(),
                ))
            }))
        );

        let destinations_yaml = r#"
        name: json patch to destinations
        destinations:
          actual: http://localhost:3000
          expect: http://localhost:3000
        testcase:
        - description: test description
          target: /information
          setting:
            json:
              patch:
                actual:
                - op: remove
                  path: /datetime
                expect:
                - op: remove
                  path: /datetime
        "#;
        let config = Config::read_str(destinations_yaml, Format::Yaml).unwrap();
        assert_eq!(
            config.testcase[0].setting.evaluate,
            Some(Evaluate::Json(JsonEvaluate {
                ignore: vec![],
                patch: Some(PatchTo::Destinations(Destinations::from([
                    (
                        "actual".to_string(),
                        serde_json::from_value(serde_json::json!([{"op": "remove", "path": "/datetime"}])).unwrap(),
                    ),
                    (
                        "expect".to_string(),
                        serde_json::from_value(serde_json::json!([{"op": "remove", "path": "/datetime"}])).unwrap(),
                    ),
                ])))
            }))
        );
    }
}
