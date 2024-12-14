pub trait Transpose {
    type Output;
    fn transpose(self) -> Self::Output;
}

pub mod transpose_template_serde {
    use std::collections::HashMap;

    use serde::{Deserializer, Serializer};

    use crate::{interface::template::Template, service::destinations::Destinations};

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
