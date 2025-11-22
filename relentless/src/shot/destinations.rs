use std::{
    collections::HashMap,
    ops::{Deref, DerefMut},
};

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "kebab-case")]
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
impl<T> IntoIterator for Destinations<T> {
    type Item = <HashMap<String, T> as IntoIterator>::Item;
    type IntoIter = <HashMap<String, T> as IntoIterator>::IntoIter;
    fn into_iter(self) -> Self::IntoIter {
        self.0.into_iter()
    }
}
impl<T> Extend<(String, T)> for Destinations<T> {
    fn extend<I: IntoIterator<Item = (String, T)>>(&mut self, iter: I) {
        self.0.extend(iter)
    }
}
