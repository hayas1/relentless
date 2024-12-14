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
