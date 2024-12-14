use std::{
    collections::HashMap,
    fmt::Debug,
    fs::{read_to_string, File},
    path::Path,
    time::Duration,
};

use destinations::Destinations;
use serde::{de::DeserializeOwned, Deserialize, Serialize};

use crate::{
    error::{RunCommandError, WrappedResult},
    interface::template::Template,
};

use super::helper::{coalesce::Coalesce, http_serde_priv, is_default::IsDefault};

// TODO this trait should be divided
pub trait Configuration: Debug + Clone + PartialEq + Eq + Serialize + DeserializeOwned + Default {}
impl<T> Configuration for T where T: Debug + Clone + PartialEq + Eq + Serialize + DeserializeOwned + Default {}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(deny_unknown_fields, rename_all = "kebab-case")]
#[serde(bound = "Q: Configuration, P: Configuration")]
pub struct Config<Q, P> {
    #[serde(flatten, default, skip_serializing_if = "IsDefault::is_default")]
    pub worker_config: WorkerConfig<Q, P>,

    #[serde(default, skip_serializing_if = "IsDefault::is_default")]
    pub testcases: Vec<Testcase<Q, P>>,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
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
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(deny_unknown_fields, rename_all = "kebab-case")]
#[serde(bound = "Q: Configuration, P: Configuration")]
pub struct Setting<Q, P> {
    #[serde(default, skip_serializing_if = "IsDefault::is_default")]
    pub request: Q,

    #[serde(default, skip_serializing_if = "IsDefault::is_default", with = "destinations::transpose_template_serde")]
    pub template: Destinations<Template>,
    #[serde(default, skip_serializing_if = "IsDefault::is_default")]
    pub repeat: Repeat,
    #[serde(default, skip_serializing_if = "IsDefault::is_default")]
    pub timeout: Option<Duration>, // TODO parse from string? https://crates.io/crates/humantime ?

    #[serde(default, skip_serializing_if = "IsDefault::is_default")]
    pub response: P,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
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
#[serde(bound = "Q: Configuration, P: Configuration")]
pub struct Testcase<Q, P> {
    #[serde(default, skip_serializing_if = "IsDefault::is_default")]
    pub description: Option<String>,
    pub target: String,

    #[serde(default, skip_serializing_if = "IsDefault::is_default")]
    pub setting: Setting<Q, P>,
    #[serde(default, skip_serializing_if = "IsDefault::is_default")]
    pub attr: Attribute,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(deny_unknown_fields, rename_all = "kebab-case")]
pub struct Attribute {
    #[serde(default, skip_serializing_if = "IsDefault::is_default")]
    pub allow: bool,
}

impl<Q: Configuration, P: Configuration> Config<Q, P> {
    pub fn read<A: AsRef<Path>>(path: A) -> WrappedResult<Self> {
        Ok(Format::from_path(path.as_ref())?
            .deserialize_testcase(path.as_ref())
            .map_err(|e| e.context(path.as_ref().display().to_string()))?)
    }
    pub fn read_str(s: &str, format: Format) -> WrappedResult<Self> {
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
            response: self.response.coalesce(&other.response),
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
    pub fn from_path<A: AsRef<Path>>(path: A) -> WrappedResult<Self> {
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

    pub fn deserialize_testcase<A: AsRef<Path>, Q: Configuration, P: Configuration>(
        &self,
        path: A,
    ) -> WrappedResult<Config<Q, P>> {
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

    pub fn deserialize_testcase_str<Q: Configuration, P: Configuration>(
        &self,
        content: &str,
    ) -> WrappedResult<Config<Q, P>> {
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

pub mod destinations {
    use std::{
        collections::{
            hash_map::{IntoIter as HashMapIter, IntoKeys, IntoValues},
            HashMap,
        },
        hash::Hash,
        ops::{Deref, DerefMut},
    };

    use serde::{Deserialize, Serialize};

    #[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
    pub struct Destinations<T>(HashMap<String, T>);
    impl<T> Default for Destinations<T> {
        fn default() -> Self {
            // derive(Default) do not implement Default when T are not implement Default
            // https://github.com/rust-lang/rust/issues/26925
            Self(HashMap::new())
        }
    }
    impl<T> Deref for Destinations<T> {
        type Target = HashMap<String, T>;
        fn deref(&self) -> &Self::Target {
            &self.0
        }
    }
    impl<T> DerefMut for Destinations<T> {
        fn deref_mut(&mut self) -> &mut Self::Target {
            &mut self.0
        }
    }
    impl<T> FromIterator<(String, T)> for Destinations<T> {
        fn from_iter<I: IntoIterator<Item = (String, T)>>(iter: I) -> Self {
            Self(iter.into_iter().collect())
        }
    }
    impl<T> IntoIterator for Destinations<T> {
        type Item = (String, T);
        type IntoIter = HashMapIter<String, T>;
        fn into_iter(self) -> Self::IntoIter {
            self.0.into_iter()
        }
    }
    impl<T> From<HashMap<String, T>> for Destinations<T> {
        fn from(dest: HashMap<String, T>) -> Self {
            Self(dest)
        }
    }
    impl<T> From<Destinations<T>> for HashMap<String, T> {
        fn from(dest: Destinations<T>) -> Self {
            dest.0
        }
    }

    impl<T> Destinations<T> {
        pub fn new() -> Self {
            Default::default()
        }

        pub fn into_keys(self) -> IntoKeys<String, T> {
            self.0.into_keys()
        }

        pub fn into_values(self) -> IntoValues<String, T> {
            self.0.into_values()
        }
    }
    pub trait Transpose {
        type Output;
        fn transpose(self) -> Self::Output;
    }
    impl<T> Transpose for Destinations<Vec<T>> {
        type Output = Vec<Destinations<T>>;
        fn transpose(self) -> Self::Output {
            let mut t = Vec::new();
            for (k, it) in self {
                for (i, v) in it.into_iter().enumerate() {
                    if t.len() <= i {
                        t.push(Destinations::from_iter([(k.clone(), v)]));
                    } else {
                        t[i].insert(k.clone(), v);
                    }
                }
            }
            t
        }
    }
    impl<T> Transpose for Vec<Destinations<T>> {
        type Output = Destinations<Vec<T>>;
        fn transpose(self) -> Self::Output {
            let mut t = Destinations::new();
            for d in self {
                for (k, v) in d {
                    t.entry(k).or_insert_with(Vec::new).push(v);
                }
            }
            t
        }
    }

    impl<K, V> Transpose for Destinations<HashMap<K, V>>
    where
        K: Hash + Eq + Clone,
    {
        type Output = HashMap<K, Destinations<V>>;
        fn transpose(self) -> Self::Output {
            let mut t = HashMap::new();
            for (k, v) in self {
                for (dest, i) in v {
                    t.entry(dest).or_insert_with(Destinations::new).insert(k.clone(), i);
                }
            }
            t
        }
    }
    impl<K, V> Transpose for HashMap<K, Destinations<V>>
    where
        K: Hash + Eq + Clone,
    {
        type Output = Destinations<HashMap<K, V>>;
        fn transpose(self) -> Self::Output {
            let mut t = Destinations::new();
            for (k, d) in self {
                for (dest, v) in d {
                    t.entry(dest).or_insert_with(HashMap::new).insert(k.clone(), v);
                }
            }
            t
        }
    }

    pub mod transpose_template_serde {
        use std::collections::HashMap;

        use serde::{Deserializer, Serializer};

        use crate::interface::template::Template;

        use super::Destinations;

        pub fn serialize<S>(template: &Destinations<Template>, serializer: S) -> Result<S::Ok, S::Error>
        where
            S: Serializer,
        {
            super::transpose_serde::serialize(
                &template
                    .clone()
                    .into_iter()
                    .map(|(d, t)| (d, t.into_iter().collect()))
                    .collect::<Destinations<HashMap<String, String>>>(),
                serializer,
            )
        }

        pub fn deserialize<'de, D>(deserializer: D) -> Result<Destinations<Template>, D::Error>
        where
            D: Deserializer<'de>,
        {
            super::transpose_serde::deserialize::<Destinations<HashMap<String, String>>, _>(deserializer)
                .map(|templates| templates.into_iter().map(|(d, t)| (d, t.into_iter().collect())).collect())
        }
    }

    pub mod transpose_serde {
        use serde::{Deserialize, Deserializer, Serialize, Serializer};

        use super::Transpose;

        pub fn serialize<T, S>(transpose: &T, serializer: S) -> Result<S::Ok, S::Error>
        where
            T: Clone + Transpose,
            T::Output: Serialize,
            S: Serializer,
        {
            transpose.clone().transpose().serialize(serializer)
        }

        pub fn deserialize<'de, T, D>(deserializer: D) -> Result<T::Output, D::Error>
        where
            T: Deserialize<'de> + Transpose,
            D: Deserializer<'de>,
        {
            T::deserialize(deserializer).map(Transpose::transpose)
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::service::impl_http::{
        evaluate::{BodyEvaluate, HeaderEvaluate, HttpResponse, JsonEvaluate},
        factory::HttpRequest,
    };

    use super::*;

    #[test]
    #[cfg(not(any(feature = "json", feature = "yaml", feature = "toml")))]
    fn test_no_default_features() {
        let err = Config::<HttpRequest, HttpResponse>::read("path/to/config.yaml").unwrap_err();
        assert_eq!(err.downcast_ref(), Some(&RunCommandError::UnknownFormatExtension("yaml".to_string())));
    }

    #[test]
    #[cfg(all(feature = "yaml", feature = "json"))]
    fn test_config_roundtrip() {
        let example = Config {
            worker_config: WorkerConfig {
                name: Some("example".to_string()),
                setting: Setting {
                    request: HttpRequest::default(),
                    response: HttpResponse { header: HeaderEvaluate::Ignore, ..Default::default() },
                    ..Default::default()
                },
                ..Default::default()
            },
            testcases: vec![Testcase {
                description: Some("test description".to_string()),
                target: "/information".to_string(),
                setting: Setting {
                    request: HttpRequest::default(),
                    response: HttpResponse {
                        body: BodyEvaluate::Json(JsonEvaluate {
                            ignore: vec!["/datetime".to_string()],
                            // patch: Some(PatchTo::All(
                            //     serde_json::from_value(
                            //         serde_json::json!([{"op": "replace", "path": "/datetime", "value": "2021-01-01"}]),
                            //     )
                            //     .unwrap(),
                            // )),
                            patch: Some(EvaluateTo::Destinations(Destinations::from_iter([
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
            response:
              body:
                json:
                  patch:
                  - op: replace
                    path: /datetime
                    value: 2021-01-01
        "#;
        let config = Config::read_str(all_yaml, Format::Yaml).unwrap();
        assert_eq!(
            config.testcases[0].setting,
            Setting {
                request: HttpRequest::default(),
                response: HttpResponse {
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
            response:
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
            config.testcases[0].setting,
            Setting {
                request: HttpRequest::default(),
                response: HttpResponse {
                    body: BodyEvaluate::Json(JsonEvaluate {
                        ignore: vec![],
                        patch: Some(EvaluateTo::Destinations(Destinations::from_iter([
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
                        patch_fail: Some(Severity::Warn),
                    }),
                    ..Default::default()
                },
                ..Default::default()
            },
        );
    }
}
