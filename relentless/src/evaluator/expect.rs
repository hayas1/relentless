use serde::{Deserialize, Serialize};

use crate::assault::destinations::Destinations;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "kebab-case")]
pub enum ExpectEvaluator<T> {
    All(T),
    Destinations(Destinations<T>),
}
