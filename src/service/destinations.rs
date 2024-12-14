use std::{
    collections::{
        hash_map::{IntoIter as HashMapIter, IntoKeys, IntoValues},
        HashMap,
    },
    hash::Hash,
    ops::{Deref, DerefMut},
};

use serde::{Deserialize, Serialize};

use crate::interface::helper::transpose::Transpose;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "kebab-case", untagged)]
pub enum EvaluateTo<T> {
    All(T),
    Destinations(Destinations<T>),
}

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
impl<S: ToString, T> FromIterator<(S, T)> for Destinations<T> {
    fn from_iter<I: IntoIterator<Item = (S, T)>>(iter: I) -> Self {
        Self(iter.into_iter().map(|(k, val)| (k.to_string(), val)).collect())
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_transpose_destination_vec_roundtrip() {
        let d: Destinations<_> = [("a", vec![1, 2]), ("b", vec![3, 4]), ("c", vec![5, 6])].into_iter().collect();
        let transposed = vec![
            [("a", 1), ("b", 3), ("c", 5)].into_iter().collect(),
            [("a", 2), ("b", 4), ("c", 6)].into_iter().collect(),
        ];

        let t = d.clone().transpose();
        assert_eq!(t, transposed);
        assert_eq!(t.transpose(), d);
    }

    #[test]
    fn test_transpose_destination_hashmap_roundtrip() {
        let d: Destinations<HashMap<_, _>> = [
            ("a", vec![(1, "1"), (2, "2")].into_iter().collect()),
            ("b", vec![(1, "one"), (2, "two")].into_iter().collect()),
            ("c", vec![(1, "一"), (2, "二")].into_iter().collect()),
        ]
        .into_iter()
        .collect();
        let transposed = vec![
            (1, [("a", "1"), ("b", "one"), ("c", "一")].into_iter().collect()),
            (2, [("a", "2"), ("b", "two"), ("c", "二")].into_iter().collect()),
        ]
        .into_iter()
        .collect();

        let t = d.clone().transpose();
        assert_eq!(t, transposed);
        assert_eq!(t.transpose(), d);
    }

    // TODO if length is not equal
}
