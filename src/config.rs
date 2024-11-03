use std::{
    collections::HashMap,
    fs::{read_to_string, File},
    path::Path,
    time::Duration,
};

use serde::{Deserialize, Serialize};

use crate::error::{RunCommandError, WrappedResult};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(deny_unknown_fields, rename_all = "kebab-case")]
pub struct Config {
    #[serde(flatten, default, skip_serializing_if = "IsDefault::is_default")]
    pub worker_config: WorkerConfig,

    #[serde(default, skip_serializing_if = "IsDefault::is_default")]
    pub testcases: Vec<Testcase>,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(deny_unknown_fields, rename_all = "kebab-case")]
pub struct WorkerConfig {
    #[serde(default, skip_serializing_if = "IsDefault::is_default")]
    pub name: Option<String>,
    #[serde(default, skip_serializing_if = "IsDefault::is_default")]
    pub destinations: Destinations<http_serde_priv::Uri>,
    #[serde(default, skip_serializing_if = "IsDefault::is_default")]
    pub setting: Setting,
}
pub type Destinations<T> = HashMap<String, T>;
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(deny_unknown_fields, rename_all = "kebab-case")]
pub struct Setting {
    #[serde(default, skip_serializing_if = "IsDefault::is_default")]
    pub request: RequestInfo,
    #[serde(default, skip_serializing_if = "IsDefault::is_default")]
    pub template: HashMap<String, Destinations<String>>,
    #[serde(default, skip_serializing_if = "IsDefault::is_default")]
    pub repeat: Repeat,
    #[serde(default, skip_serializing_if = "IsDefault::is_default")]
    pub timeout: Option<Duration>,
    #[serde(default, skip_serializing_if = "IsDefault::is_default")]
    #[cfg_attr(feature = "yaml", serde(with = "serde_yaml::with::singleton_map_recursive"))]
    pub evaluate: Evaluate,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(deny_unknown_fields, rename_all = "kebab-case")]
pub struct RequestInfo {
    #[serde(default, skip_serializing_if = "IsDefault::is_default")]
    pub method: Option<http_serde_priv::Method>,
    #[serde(default, skip_serializing_if = "IsDefault::is_default")]
    pub headers: Option<http_serde_priv::HeaderMap>,
    #[serde(default, skip_serializing_if = "IsDefault::is_default")]
    pub body: Option<BodyStructure>,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(deny_unknown_fields, rename_all = "kebab-case", untagged)]
pub enum BodyStructure {
    #[default]
    Empty,
    Text(String),
    #[cfg(feature = "json")]
    Json(HashMap<String, String>),
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(deny_unknown_fields, rename_all = "kebab-case")]
pub struct Repeat(pub Option<usize>);
impl Coalesce for Repeat {
    type Other = Self;
    fn coalesce(self, other: &Self::Other) -> Self {
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

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(deny_unknown_fields, rename_all = "kebab-case")]
pub struct Evaluate {
    #[serde(default, skip_serializing_if = "IsDefault::is_default")]
    pub status: StatusEvaluate,
    #[serde(default, skip_serializing_if = "IsDefault::is_default")]
    pub header: HeaderEvaluate,
    #[serde(default, skip_serializing_if = "IsDefault::is_default")]
    pub body: BodyEvaluate,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(deny_unknown_fields, rename_all = "kebab-case")]
pub enum StatusEvaluate {
    #[default]
    OkOrEqual,
    Expect(EvaluateTo<http_serde_priv::StatusCode>),
    Ignore,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(deny_unknown_fields, rename_all = "kebab-case")]
pub enum HeaderEvaluate {
    #[default]
    Equal,
    Expect(EvaluateTo<http_serde_priv::HeaderMap>),
    Ignore,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(deny_unknown_fields, rename_all = "kebab-case")]
pub enum BodyEvaluate {
    #[default]
    Equal,
    PlainText(PlainTextEvaluate),
    #[cfg(feature = "json")]
    Json(JsonEvaluate),
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(deny_unknown_fields, rename_all = "kebab-case")]
pub struct PlainTextEvaluate {}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(deny_unknown_fields, rename_all = "kebab-case")]
#[cfg(feature = "json")]
pub struct JsonEvaluate {
    #[serde(default, skip_serializing_if = "IsDefault::is_default")]
    pub ignore: Vec<String>,
    #[serde(default, skip_serializing_if = "IsDefault::is_default")]
    pub patch: Option<EvaluateTo<json_patch::Patch>>,
    #[serde(default, skip_serializing_if = "IsDefault::is_default")]
    pub patch_fail: Option<Severity>,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "kebab-case", untagged)]
pub enum EvaluateTo<T> {
    All(T),
    Destinations(Destinations<T>),
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "kebab-case")]
pub enum Severity {
    Allow,
    Warn,
    Error,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(deny_unknown_fields, rename_all = "kebab-case")]
pub struct Testcase {
    #[serde(default, skip_serializing_if = "IsDefault::is_default")]
    pub description: Option<String>,
    pub target: String,

    #[serde(default, skip_serializing_if = "IsDefault::is_default")]
    pub setting: Setting,
    #[serde(default, skip_serializing_if = "IsDefault::is_default")]
    pub attr: Attribute,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(deny_unknown_fields, rename_all = "kebab-case")]
pub struct Attribute {
    #[serde(default, skip_serializing_if = "IsDefault::is_default")]
    pub allow: bool,
}

pub trait IsDefault: Default + PartialEq<Self> {
    fn is_default(&self) -> bool {
        self == &Self::default()
    }
}
impl<T> IsDefault for T where T: Default + PartialEq<T> {}

impl Config {
    pub fn read<P: AsRef<Path>>(path: P) -> WrappedResult<Self> {
        Ok(Format::from_path(path.as_ref())?
            .deserialize_testcase(path.as_ref())
            .map_err(|e| e.context(path.as_ref().display().to_string()))?)
    }
    pub fn read_str(s: &str, format: Format) -> WrappedResult<Self> {
        format.deserialize_testcase_str(s)
    }
}
impl Coalesce for WorkerConfig {
    type Other = Destinations<http_serde_priv::Uri>;
    fn coalesce(self, other: &Self::Other) -> Self {
        let destinations = self.destinations.coalesce(&other.iter().map(|(k, v)| (k.to_string(), v.clone())).collect());
        Self { destinations, ..self }
    }
}
impl<T: Clone> Coalesce for Destinations<T> {
    type Other = HashMap<String, T>;
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
            request: self.request.coalesce(&other.request),
            template: if self.template.is_empty() { other.clone().template } else { self.template },
            repeat: self.repeat.coalesce(&other.repeat),
            timeout: self.timeout.or(other.timeout),
            evaluate: self.evaluate.coalesce(&other.evaluate),
        }
    }
}
impl Coalesce for RequestInfo {
    type Other = Self;
    fn coalesce(self, other: &Self) -> Self {
        Self {
            method: self.method.or(other.method.clone()),
            headers: self.headers.or(other.headers.clone()),
            body: self.body.or(other.body.clone()),
        }
    }
}
impl Coalesce for Evaluate {
    type Other = Self;
    fn coalesce(self, other: &Self) -> Self {
        Self {
            status: self.status.coalesce(&other.status),
            header: self.header.coalesce(&other.header),
            body: self.body.coalesce(&other.body),
        }
    }
}
impl Coalesce for StatusEvaluate {
    type Other = Self;
    fn coalesce(self, other: &Self) -> Self {
        if self.is_default() {
            other.clone()
        } else {
            self
        }
    }
}
impl Coalesce for HeaderEvaluate {
    type Other = Self;
    fn coalesce(self, other: &Self) -> Self {
        if self.is_default() {
            other.clone()
        } else {
            self
        }
    }
}
impl Coalesce for BodyEvaluate {
    type Other = Self;
    fn coalesce(self, other: &Self) -> Self {
        if self.is_default() {
            other.clone()
        } else {
            self
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
    pub fn from_path<P: AsRef<Path>>(path: P) -> WrappedResult<Self> {
        let basename = path.as_ref().extension().and_then(|ext| ext.to_str());
        match basename {
            #[cfg(feature = "json")]
            Some("json") => Ok(Format::Json),
            #[cfg(feature = "yaml")]
            Some("yaml" | "yml") => Ok(Format::Yaml),
            #[cfg(feature = "toml")]
            Some("toml") => Ok(Format::Toml),
            Some(ext) => Err(RunCommandError::UnknownFormatExtension(ext.to_string()))?,
            _ => Err(RunCommandError::CannotSpecifyFormat)?,
        }
    }

    pub fn deserialize_testcase<P: AsRef<Path>>(&self, path: P) -> WrappedResult<Config> {
        match self {
            #[cfg(feature = "json")]
            Format::Json => Ok(serde_json::from_reader(File::open(path)?)?),
            #[cfg(feature = "yaml")]
            Format::Yaml => Ok(serde_yaml::from_reader(File::open(path)?)?),
            #[cfg(feature = "toml")]
            Format::Toml => Ok(toml::from_str(&read_to_string(path)?)?),
            #[cfg(not(any(feature = "json", feature = "yaml", feature = "toml")))]
            _ => {
                use crate::error::WithContext;
                Err(RunCommandError::UndefinedSerializeFormat).context(path.as_ref().display().to_string())?
            }
        }
    }

    pub fn deserialize_testcase_str(&self, content: &str) -> WrappedResult<Config> {
        match self {
            #[cfg(feature = "json")]
            Format::Json => Ok(serde_json::from_str(content)?),
            #[cfg(feature = "yaml")]
            Format::Yaml => Ok(serde_yaml::from_str(content)?),
            #[cfg(feature = "toml")]
            Format::Toml => Ok(toml::from_str(content)?),
            #[cfg(not(any(feature = "json", feature = "yaml", feature = "toml")))]
            _ => {
                use crate::error::WithContext;
                Err(RunCommandError::UndefinedSerializeFormat).context(content.to_string())?
            }
        }
    }
}

// `http` do not support serde https://github.com/hyperium/http/pull/631
pub(crate) mod http_serde_priv {
    use std::ops::{Deref, DerefMut};

    use super::*;

    #[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
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

    #[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
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

    #[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
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

    #[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
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

    #[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
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
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[cfg(not(any(feature = "json", feature = "yaml", feature = "toml")))]
    fn test_no_default_features() {
        let err = Config::read("path/to/config.yaml").unwrap_err();
        assert_eq!(err.downcast_ref(), Some(&RunCommandError::UnknownFormatExtension("yaml".to_string())));
    }

    #[test]
    #[cfg(all(feature = "yaml", feature = "toml"))]
    fn test_read_basic_config() {
        let assault_yaml = Config::read("tests/config/basic/assault.yaml").unwrap();
        let assault_toml = Config::read("tests/config/basic/assault.toml").unwrap();
        assert_json_diff::assert_json_eq!(assault_yaml, assault_toml);
        assert_eq!(assault_yaml, assault_toml);

        let compare_yaml = Config::read("tests/config/basic/compare.yaml").unwrap();
        let compare_toml = Config::read("tests/config/basic/compare.toml").unwrap();
        assert_json_diff::assert_json_eq!(compare_yaml, compare_toml);
        assert_eq!(compare_yaml, compare_toml);
    }

    #[test]
    #[cfg(all(feature = "yaml", feature = "json", feature = "toml"))]
    fn test_read_basic_config_for_json() {
        let assault_yaml = Config::read("tests/config/basic/assault_json.yaml").unwrap();
        let assault_toml = Config::read("tests/config/basic/assault_json.toml").unwrap();
        assert_json_diff::assert_json_eq!(assault_yaml, assault_toml);
        assert_eq!(assault_yaml, assault_toml);

        let compare_yaml = Config::read("tests/config/basic/compare_json.yaml").unwrap();
        let compare_toml = Config::read("tests/config/basic/compare_json.toml").unwrap();
        assert_json_diff::assert_json_eq!(compare_yaml, compare_toml);
        assert_eq!(compare_yaml, compare_toml);
    }

    #[test]
    #[cfg(all(feature = "yaml", feature = "json"))]
    fn test_config_roundtrip() {
        let example = Config {
            worker_config: WorkerConfig {
                name: Some("example".to_string()),
                setting: Setting {
                    evaluate: Evaluate { header: HeaderEvaluate::Ignore, ..Default::default() },
                    ..Default::default()
                },
                ..Default::default()
            },
            testcases: vec![Testcase {
                description: Some("test description".to_string()),
                target: "/information".to_string(),
                setting: Setting {
                    evaluate: Evaluate {
                        body: BodyEvaluate::Json(JsonEvaluate {
                            ignore: vec!["/datetime".to_string()],
                            // patch: Some(PatchTo::All(
                            //     serde_json::from_value(
                            //         serde_json::json!([{"op": "replace", "path": "/datetime", "value": "2021-01-01"}]),
                            //     )
                            //     .unwrap(),
                            // )),
                            patch: Some(EvaluateTo::Destinations(Destinations::from([
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
                            patch_fail: Some(Severity::Error),
                        }),
                        ..Default::default()
                    },
                    ..Default::default()
                },
                attr: Attribute { allow: true },
            }],
        };
        let yaml = serde_yaml::to_string(&example).unwrap();
        // println!("{}", yaml);

        let round_trip = Config::read_str(&yaml, Format::Yaml).unwrap();
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
        testcases:
        - description: test description
          target: /information
          setting:
            evaluate:
              body:
                json:
                  patch:
                  - op: replace
                    path: /datetime
                    value: 2021-01-01
        "#;
        let config = Config::read_str(all_yaml, Format::Yaml).unwrap();
        assert_eq!(
            config.testcases[0].setting.evaluate,
            Evaluate {
                body: BodyEvaluate::Json(JsonEvaluate {
                    ignore: vec![],
                    patch: Some(EvaluateTo::All(
                        serde_json::from_value(
                            serde_json::json!([{"op": "replace", "path": "/datetime", "value": "2021-01-01"}])
                        )
                        .unwrap(),
                    )),
                    patch_fail: None,
                }),
                ..Default::default()
            },
        );

        let destinations_yaml = r#"
        name: json patch to destinations
        destinations:
          actual: http://localhost:3000
          expect: http://localhost:3000
        testcases:
        - description: test description
          target: /information
          setting:
            evaluate:
              body:
                json:
                  patch:
                    actual:
                    - op: remove
                      path: /datetime
                    expect:
                    - op: remove
                      path: /datetime
                  patch-fail: warn
        "#;
        let config = Config::read_str(destinations_yaml, Format::Yaml).unwrap();
        assert_eq!(
            config.testcases[0].setting.evaluate,
            Evaluate {
                body: BodyEvaluate::Json(JsonEvaluate {
                    ignore: vec![],
                    patch: Some(EvaluateTo::Destinations(Destinations::from([
                        (
                            "actual".to_string(),
                            serde_json::from_value(serde_json::json!([{"op": "remove", "path": "/datetime"}])).unwrap(),
                        ),
                        (
                            "expect".to_string(),
                            serde_json::from_value(serde_json::json!([{"op": "remove", "path": "/datetime"}])).unwrap(),
                        ),
                    ]))),
                    patch_fail: Some(Severity::Warn),
                }),
                ..Default::default()
            },
        );
    }
}
