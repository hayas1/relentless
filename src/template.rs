use std::{
    collections::HashMap,
    ops::{Deref, DerefMut},
};

use nom::{
    branch::alt,
    bytes::complete::{is_not, tag, take_until},
    combinator::map_res,
    multi::many0,
    sequence::delimited,
    IResult,
};

use crate::error::{TemplateError, WrappedResult};

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Template {
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
impl<R: ToString, L: ToString> FromIterator<(R, L)> for Template {
    fn from_iter<I: IntoIterator<Item = (R, L)>>(iter: I) -> Self {
        Self { vars: iter.into_iter().map(|(var, val)| (var.to_string(), val.to_string())).collect() }
    }
}

impl Template {
    pub fn new() -> Self {
        Default::default()
    }

    pub fn render(&self, input: &str) -> WrappedResult<String> {
        let variables = Variable::split(input)?;
        let assigned = variables.iter().map(|v| v.assign(self)).collect::<Result<Vec<_>, _>>()?;
        Ok(assigned.join(""))
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Variable {
    Literal(String),
    Defined(String),
    // TODO Input(String),
    // TODO File(String),
    Environment(String),
}

impl Variable {
    pub fn split(input: &str) -> WrappedResult<Vec<Self>> {
        Ok(Self::parse(input).map_err(|e| TemplateError::NomParseError(e.to_string()))?.1)
    }

    pub fn assign(&self, defined: &Template) -> WrappedResult<String> {
        match self {
            Self::Literal(text) => Ok(text.clone()),
            Self::Defined(key) => Ok(defined.get(key).ok_or(TemplateError::VariableNotDefined(key.clone()))?.clone()),
            Self::Environment(key) => Ok(std::env::var(key)?),
        }
    }

    pub fn parse_environment_variable(input: &str) -> IResult<&str, Self> {
        map_res(delimited(alt((tag("${ENV:"), tag("${env:"))), take_until("}"), tag("}")), |key: &str| {
            Ok::<_, ()>(Self::Environment(key.to_string()))
        })(input)
    }

    pub fn parse_variable(input: &str) -> IResult<&str, Self> {
        map_res(delimited(tag("${"), take_until("}"), tag("}")), |key: &str| {
            Ok::<_, ()>(Self::Defined(key.to_string()))
        })(input)
    }

    pub fn parse_literal(input: &str) -> IResult<&str, Self> {
        map_res(is_not("${"), |text: &str| Ok::<_, ()>(Self::Literal(text.to_string())))(input)
    }

    pub fn parse(input: &str) -> IResult<&str, Vec<Self>> {
        many0(alt((Self::parse_environment_variable, Self::parse_variable, Self::parse_literal)))(input)
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
        if let Some(TemplateError::VariableNotDefined(var)) = error.downcast_ref::<TemplateError>() {
            assert_eq!(var, "fuga");
        }
    }
}
