use std::{
    collections::{hash_map::IntoIter as HashMapIter, HashMap},
    convert::Infallible,
    ops::{Deref, DerefMut},
};

use nom::{
    branch::alt,
    bytes::complete::{is_not, tag},
    character::complete::alphanumeric1,
    combinator::map_res,
    multi::many0,
    sequence::delimited,
    IResult, Parser,
};
use serde::{Deserialize, Serialize};
#[cfg(feature = "json")]
use serde_json::Value;

use crate::error::TemplateError;

#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct Template {
    #[serde(flatten)]
    vars: HashMap<String, String>,
}

impl Deref for Template {
    type Target = HashMap<String, String>;
    fn deref(&self) -> &Self::Target {
        &self.vars
    }
}
impl DerefMut for Template {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.vars
    }
}
impl IntoIterator for Template {
    type Item = (String, String);
    type IntoIter = HashMapIter<String, String>;
    fn into_iter(self) -> Self::IntoIter {
        self.vars.into_iter()
    }
}
impl<R: ToString, L: ToString> FromIterator<(R, L)> for Template {
    fn from_iter<I: IntoIterator<Item = (R, L)>>(iter: I) -> Self {
        Self { vars: iter.into_iter().map(|(var, val)| (var.to_string(), val.to_string())).collect() }
    }
}

impl Template {
    pub fn new() -> Self {
        Default::default()
    }

    pub fn render(&self, input: &str) -> crate::Result<String> {
        let variables = Variable::split(input)?;
        let assigned = variables.iter().map(|v| v.assign(self)).collect::<Result<Vec<_>, _>>()?;
        Ok(assigned.join(""))
    }

    #[cfg(feature = "json")]
    pub fn render_json_recursive(&self, input: &Value) -> crate::Result<Value> {
        match input {
            Value::Object(m) => Ok(Value::Object(
                m.iter()
                    .map(|(k, v)| self.render_json_recursive(v).map(|v| (k.clone(), v)))
                    .collect::<Result<_, _>>()?,
            )),
            Value::Array(v) => {
                Ok(Value::Array(v.iter().map(|v| self.render_json_recursive(v)).collect::<Result<_, _>>()?))
            }
            Value::String(s) => Ok(Value::String(self.render(s)?)),
            Value::Number(n) => Ok(Value::Number(n.clone())),
            Value::Bool(b) => Ok(Value::Bool(*b)),
            Value::Null => Ok(Value::Null),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Variable {
    Literal(String),
    Defined(String),
    Environment(String),
}

impl Variable {
    pub fn split(input: &str) -> crate::Result<Vec<Self>> {
        let (remain, parsed) = Self::parse(input).map_err(|e| TemplateError::NomParseError(e.to_string()))?;
        remain
            .is_empty()
            .then_some(parsed)
            .ok_or_else(|| crate::Error::from(TemplateError::RemainingTemplate(remain.to_string())))
    }

    pub fn assign(&self, defined: &Template) -> crate::Result<String> {
        match self {
            Self::Literal(text) => Ok(text.clone()),
            Self::Defined(key) => {
                Ok(defined.get(key).ok_or(TemplateError::VariableNotDefined(key.clone()))?.clone())
            }
            Self::Environment(key) => {
                std::env::var(key).map_err(|e| crate::Error::boxed(e))
            }
        }
    }

    pub fn parse_environment_variable(input: &str) -> IResult<&str, Self> {
        let parser = delimited(alt((tag("${ENV:"), tag("${env:"))), alphanumeric1, tag("}"));
        map_res(parser, |key: &str| Ok::<_, Infallible>(Self::Environment(key.to_string()))).parse(input)
    }

    pub fn parse_variable(input: &str) -> IResult<&str, Self> {
        let parser = delimited(tag("${"), alphanumeric1, tag("}"));
        map_res(parser, |key: &str| Ok::<_, Infallible>(Self::Defined(key.to_string()))).parse(input)
    }

    pub fn parse_literal(input: &str) -> IResult<&str, Self> {
        let parser = is_not("$");
        map_res(parser, |text: &str| Ok::<_, Infallible>(Self::Literal(text.to_string()))).parse(input)
    }

    /// Match a lone `$` that is not the start of `${` — treated as a literal.
    pub fn parse_lone_dollar(input: &str) -> IResult<&str, Self> {
        let (input, _) = tag("$")(input)?;
        let (input, _) = nom::combinator::not(tag("{"))(input)?;
        Ok((input, Self::Literal("$".to_string())))
    }

    pub fn parse(input: &str) -> IResult<&str, Vec<Self>> {
        let parser = alt((
            Self::parse_environment_variable,
            Self::parse_variable,
            Self::parse_literal,
            Self::parse_lone_dollar,
        ));
        many0(parser).parse(input)
    }
}

/// Serde module for deserializing `{var_name -> {dest_name -> value}}` into `Destinations<Template>`
/// (transposing to `{dest_name -> {var_name -> value}}`).
pub mod destinations_serde {
    use std::collections::HashMap;

    use serde::{Deserialize, Deserializer, Serialize, Serializer};

    use crate::shot::destinations::Destinations;

    use super::Template;

    pub fn serialize<S>(template: &Destinations<Template>, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let transposed: HashMap<String, HashMap<String, String>> =
            template.iter().flat_map(|(dest, t)| t.iter().map(move |(var, val)| (var.clone(), dest.clone(), val.clone()))).fold(
                HashMap::new(),
                |mut acc, (var, dest, val)| {
                    acc.entry(var).or_default().insert(dest, val);
                    acc
                },
            );
        transposed.serialize(serializer)
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<Destinations<Template>, D::Error>
    where
        D: Deserializer<'de>,
    {
        let by_var: HashMap<String, HashMap<String, String>> = HashMap::deserialize(deserializer)?;
        let mut by_dest: HashMap<String, HashMap<String, String>> = HashMap::new();
        for (var, dests) in by_var {
            for (dest, val) in dests {
                by_dest.entry(dest).or_default().insert(var.clone(), val);
            }
        }
        Ok(by_dest.into_iter().map(|(dest, vars)| (dest, vars.into_iter().collect::<Template>())).collect())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_variable() {
        let input = "${foo} bar ${baz} ${ENV:SECRET}";
        let parsed = Variable::split(input).unwrap();
        assert_eq!(
            parsed,
            vec![
                Variable::Defined("foo".to_string()),
                Variable::Literal(" bar ".to_string()),
                Variable::Defined("baz".to_string()),
                Variable::Literal(" ".to_string()),
                Variable::Environment("SECRET".to_string()),
            ]
        );
    }

    #[test]
    fn test_template_render() {
        let template: Template = vec![("foo", "hoge"), ("bar", "fuga"), ("baz", "piyo")].into_iter().collect();
        std::env::set_var("SECRET", "VERY_SENSITIVE_VALUE");
        let rendered = template.render("${foo} bar ${baz} ${env:SECRET}").unwrap();
        assert_eq!(rendered, "hoge bar piyo VERY_SENSITIVE_VALUE".to_string());
    }

    #[test]
    fn test_template_render_with_undefined() {
        let template: Template = vec![("foo", "hoge"), ("bar", "fuga"), ("baz", "piyo")].into_iter().collect();
        let error = template.render("hoge ${fuga} piyo").unwrap_err();
        assert!(matches!(
            error,
            crate::Error::TemplateError(TemplateError::VariableNotDefined(ref var)) if var == "fuga"
        ));
    }

    #[test]
    fn test_template_render_with_invalid() {
        let template: Template = vec![("foo", "hoge"), ("bar", "fuga"), ("baz", "piyo")].into_iter().collect();
        let error = template.render("foo ${bar baz").unwrap_err();
        assert!(matches!(
            error,
            crate::Error::TemplateError(TemplateError::RemainingTemplate(ref s)) if s == "${bar baz",
        ));
    }
}
