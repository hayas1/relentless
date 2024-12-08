use std::{
    collections::HashMap,
    fs::{read_to_string, File},
    path::Path,
    time::Duration,
};

use destinations::Destinations;
use http::{
    header::{CONTENT_LENGTH, CONTENT_TYPE},
    HeaderMap,
};
use http_body::Body;
use mime::{Mime, APPLICATION_JSON, TEXT_PLAIN};
use serde::{Deserialize, Serialize};

use crate::{
    error::{RunCommandError, WrappedResult},
    service::FromBodyStructure,
    template::Template,
};

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
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(deny_unknown_fields, rename_all = "kebab-case")]
pub struct Setting {
    #[serde(flatten, skip_serializing_if = "IsDefault::is_default")]
    pub protocol: Option<Protocol>, // serde(default, flatten) will cause error https://github.com/serde-rs/serde/issues/1626

    #[serde(default, skip_serializing_if = "IsDefault::is_default", with = "destinations::transpose_template_serde")]
    pub template: Destinations<Template>,
    #[serde(default, skip_serializing_if = "IsDefault::is_default")]
    pub repeat: Repeat,
    #[serde(default, skip_serializing_if = "IsDefault::is_default")]
    pub timeout: Option<Duration>, // TODO parse from string? https://crates.io/crates/humantime ?
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "kebab-case")]
pub enum Protocol {
    Http {
        #[serde(default, skip_serializing_if = "IsDefault::is_default")]
        request: HttpRequest,
        #[serde(default, skip_serializing_if = "IsDefault::is_default")]
        #[cfg_attr(feature = "yaml", serde(with = "serde_yaml::with::singleton_map_recursive"))]
        evaluate: HttpEvaluate,
    },
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(deny_unknown_fields, rename_all = "kebab-case")]
pub struct HttpRequest {
    #[serde(default, skip_serializing_if = "IsDefault::is_default")]
    pub no_additional_headers: bool,
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
    Plaintext(String),
    #[cfg(feature = "json")]
    Json(HashMap<String, String>),
}
impl BodyStructure {
    pub fn body_with_headers<ReqB: FromBodyStructure + Body>(
        self,
        template: &Template,
    ) -> WrappedResult<(ReqB, HeaderMap)> {
        let mut headers = HeaderMap::new();
        self.content_type()
            .map(|t| headers.insert(CONTENT_TYPE, t.as_ref().parse().unwrap_or_else(|_| unreachable!())));
        let body = ReqB::from_body_structure(self, template);
        body.size_hint().exact().filter(|size| *size > 0).map(|size| headers.insert(CONTENT_LENGTH, size.into())); // TODO remove ?
        Ok((body, headers))
    }
    pub fn content_type(&self) -> Option<Mime> {
        match self {
            BodyStructure::Empty => None,
            BodyStructure::Plaintext(_) => Some(TEXT_PLAIN),
            #[cfg(feature = "json")]
            BodyStructure::Json(_) => Some(APPLICATION_JSON),
        }
    }
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
pub struct HttpEvaluate {
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
    AnyOrEqual,
    Expect(EvaluateTo<http_serde_priv::HeaderMap>),
    Ignore,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(deny_unknown_fields, rename_all = "kebab-case")]
pub enum BodyEvaluate {
    #[default]
    AnyOrEqual,
    Plaintext(EvaluateTo<PlaintextEvaluate>),
    #[cfg(feature = "json")]
    Json(JsonEvaluate),
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(deny_unknown_fields, rename_all = "kebab-case")]
pub struct PlaintextEvaluate {
    pub regex: Option<String>,
}
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
        let mut destinations = self.clone();
        for (name, dest) in other {
            destinations.entry(name.to_string()).and_modify(|d| *d = dest.clone());
        }
        destinations
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
            protocol: self.protocol.or(other.protocol.clone()),
            template: if self.template.is_empty() { other.clone().template } else { self.template },
            repeat: self.repeat.coalesce(&other.repeat),
            timeout: self.timeout.or(other.timeout),
        }
    }
}
impl Coalesce for Protocol {
    type Other = Self;
    fn coalesce(self, other: &Self) -> Self {
        match (self, other) {
            (
                Protocol::Http { request: self_request, evaluate: self_evaluate },
                Protocol::Http { request: other_request, evaluate: other_evaluate },
            ) => Protocol::Http {
                request: self_request.coalesce(other_request),
                evaluate: self_evaluate.coalesce(other_evaluate),
            },
        }
    }
}
impl Coalesce for HttpRequest {
    type Other = Self;
    fn coalesce(self, other: &Self) -> Self {
        Self {
            no_additional_headers: self.no_additional_headers || other.no_additional_headers,
            method: self.method.or(other.method.clone()),
            headers: self.headers.or(other.headers.clone()),
            body: self.body.or(other.body.clone()),
        }
    }
}
impl Coalesce for HttpEvaluate {
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

        use crate::template::Template;

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
                    protocol: Some(Protocol::Http {
                        request: Default::default(),
                        evaluate: HttpEvaluate { header: HeaderEvaluate::Ignore, ..Default::default() },
                    }),
                    ..Default::default()
                },
                ..Default::default()
            },
            testcases: vec![Testcase {
                description: Some("test description".to_string()),
                target: "/information".to_string(),
                setting: Setting {
                    protocol: Some(Protocol::Http {
                        request: Default::default(),
                        evaluate: HttpEvaluate {
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
                                        serde_json::from_value(
                                            serde_json::json!([{"op": "remove", "path": "/datetime"}]),
                                        )
                                        .unwrap(),
                                    ),
                                    (
                                        "expect".to_string(),
                                        serde_json::from_value(
                                            serde_json::json!([{"op": "remove", "path": "/datetime"}]),
                                        )
                                        .unwrap(),
                                    ),
                                ]))),
                                patch_fail: Some(Severity::Error),
                            }),
                            ..Default::default()
                        },
                    }),
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
            http:
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
            config.testcases[0].setting.protocol,
            Some(Protocol::Http {
                request: Default::default(),
                evaluate: HttpEvaluate {
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
            }),
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
            http:
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
            config.testcases[0].setting.protocol,
            Some(Protocol::Http {
                request: Default::default(),
                evaluate: HttpEvaluate {
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
            }),
        );
    }
}
