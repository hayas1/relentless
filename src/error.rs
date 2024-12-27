use std::{
    fmt::{Debug, Display},
    ops::{Deref, DerefMut},
};

use thiserror::Error;

use crate::{
    implement::service_http::{evaluate::HttpResponse, factory::HttpRequest},
    interface::config::Config,
};

pub type RelentlessResult<T, E = RelentlessError> = Result<T, E>;

#[derive(Error, Debug)]
#[error(transparent)]
pub struct RelentlessError {
    #[from]
    source: Box<dyn std::error::Error + Send + Sync>,
}
impl From<Wrap> for RelentlessError {
    fn from(wrap: Wrap) -> Self {
        RelentlessError { source: wrap.0 }
    }
}
impl<T> From<Context<T>> for RelentlessError {
    fn from(context: Context<T>) -> Self {
        let source = context.source;
        RelentlessError { source }
    }
}
impl From<crate::error2::RelentlessError> for RelentlessError {
    fn from(e: crate::error2::RelentlessError) -> Self {
        RelentlessError { source: Box::new(e) }
    }
}
impl RelentlessError {
    pub fn wrap<E>(e: E) -> Self
    where
        Wrap: From<E>,
    {
        Self::from(Wrap::from(e))
    }
    pub fn is<E: std::error::Error + Send + Sync + 'static>(&self) -> bool {
        self.source.is::<E>()
    }
    pub fn downcast<E: std::error::Error + Send + Sync + 'static>(
        self,
    ) -> Result<Box<E>, Box<dyn std::error::Error + Send + Sync>> {
        self.source.downcast()
    }
    pub fn downcast_ref<E: std::error::Error + Send + Sync + 'static>(&self) -> Option<&E> {
        self.source.downcast_ref()
    }
    pub fn downcast_mut<E: std::error::Error + Send + Sync + 'static>(&mut self) -> Option<&mut E> {
        self.source.downcast_mut()
    }
}

pub type WrappedResult<T, E = Wrap> = Result<T, E>;

#[derive(Debug)]
pub struct Wrap(pub Box<dyn std::error::Error + Send + Sync>);
impl<E: std::error::Error + Send + Sync + 'static> From<E> for Wrap {
    fn from(e: E) -> Self {
        Self::new(Box::new(e))
    }
}
impl Display for Wrap {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        Display::fmt(&*self.0, f)
    }
}
impl Deref for Wrap {
    type Target = Box<dyn std::error::Error + Send + Sync>;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}
impl DerefMut for Wrap {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}
impl Wrap {
    pub fn new(e: Box<dyn std::error::Error + Send + Sync>) -> Self {
        Self(e)
    }
    pub fn wrapping<E: std::error::Error + Send + Sync + 'static>(e: E) -> Self {
        Self::from(e)
    }
    pub fn error<E: std::error::Error + Send + Sync + 'static>(e: E) -> crate::Error {
        Self::from(e).into()
    }

    pub fn source(self) -> Box<dyn std::error::Error + Send + Sync> {
        self.0
    }
    pub fn context<T>(self, context: T) -> Context<T> {
        Context { context, source: self.0 }
    }
    pub fn is<E: std::error::Error + Send + Sync + 'static>(&self) -> bool {
        self.0.is::<E>()
    }
    pub fn downcast<E: std::error::Error + Send + Sync + 'static>(
        self,
    ) -> Result<Box<E>, Box<dyn std::error::Error + Send + Sync>> {
        self.0.downcast()
    }
    pub fn downcast_ref<E: std::error::Error + Send + Sync + 'static>(&self) -> Option<&E> {
        self.0.downcast_ref()
    }
    pub fn downcast_mut<E: std::error::Error + Send + Sync + 'static>(&mut self) -> Option<&mut E> {
        self.0.downcast_mut()
    }

    pub fn is_context<C: Display + Debug + 'static>(&self) -> bool {
        self.0.is::<Context<C>>()
    }
    pub fn downcast_context<C: Display + Debug + 'static, E: std::error::Error + Send + Sync + 'static>(
        self,
    ) -> Result<(C, Box<E>), Box<dyn std::error::Error + Send + Sync>> {
        match self.0.downcast::<Context<C>>() {
            Ok(c) => {
                let (context, source) = c.unpack();
                Ok((context, source.downcast()?))
            }
            Err(source) => Err(source),
        }
    }
    pub fn downcast_context_ref<C: Display + Debug + 'static, E: std::error::Error + Send + Sync + 'static>(
        &self,
    ) -> Option<(&C, &E)> {
        match self.0.downcast_ref::<Context<C>>() {
            Some(c) => {
                let (context, source) = c.unpack_ref();
                Some((context, source.downcast_ref()?))
            }
            None => None,
        }
    }
    pub fn downcast_context_mut<C: Display + Debug + 'static, E: std::error::Error + Send + Sync + 'static>(
        &mut self,
    ) -> Option<(&C, &mut E)> {
        match self.0.downcast_mut::<Context<C>>() {
            Some(c) => {
                let (context, source) = c.unpack_mut();
                Some((context, source.downcast_mut()?))
            }
            None => None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MultiWrap<W = Wrap>(pub Vec<W>);
impl<W: Display> Display for MultiWrap<W> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let (m, n) = (self.0.len(), 3);
        for (i, wrap) in self.0[..n.min(m)].iter().enumerate() {
            if i < n.min(m) - 1 {
                writeln!(f, "{}", wrap)?;
            } else {
                write!(f, "{}", wrap)?;
            }
        }
        if m > n {
            writeln!(f)?;
            write!(f, "... and {} more", m - n)?;
        }
        Ok(())
    }
}
impl<W: std::error::Error + 'static> std::error::Error for MultiWrap<W> {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        // TODO multiple sources ?
        self.0.first().map(|w| w as _)
    }
}
impl std::error::Error for MultiWrap<Wrap> {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        // TODO multiple sources ?
        self.0.first().map(|w| w.0.as_ref() as _)
    }
}
impl<W> FromIterator<W> for MultiWrap<W> {
    fn from_iter<T: IntoIterator<Item = W>>(iter: T) -> Self {
        Self(iter.into_iter().collect())
    }
}
impl<W> Deref for MultiWrap<W> {
    type Target = Vec<W>;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}
impl<W> DerefMut for MultiWrap<W> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

pub trait IntoContext: std::error::Error + Send + Sync {
    fn context<C>(self, context: C) -> Context<C>;
}
impl<E: std::error::Error + Send + Sync + 'static> IntoContext for E {
    fn context<C>(self, context: C) -> Context<C> {
        Context { context, source: Box::new(self) }
    }
}
#[derive(Debug)]
pub struct Context<C> {
    context: C,
    source: Box<dyn std::error::Error + Send + Sync>,
}
impl<C: Display + Debug> std::error::Error for Context<C> {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        Some(self.source.as_ref())
    }
}
impl<C: Display> Display for Context<C> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "{}:", self.context)?;
        write!(f, "{}", self.source)
    }
}
impl<C> Context<C> {
    pub fn unpack(self) -> (C, Box<dyn std::error::Error + Send + Sync>) {
        (self.context, self.source)
    }
    #[allow(clippy::borrowed_box)] // TODO
    pub fn unpack_ref(&self) -> (&C, &Box<dyn std::error::Error + Send + Sync>) {
        (&self.context, &self.source)
    }
    pub fn unpack_mut(&mut self) -> (&C, &mut Box<dyn std::error::Error + Send + Sync>) {
        (&self.context, &mut self.source)
    }
    pub fn context_ref(&self) -> &C {
        &self.context
    }
    pub fn context_mut(&mut self) -> &mut C {
        &mut self.context
    }
    pub fn downcast<E: std::error::Error + Send + Sync + 'static>(
        self,
    ) -> Result<Box<E>, Box<dyn std::error::Error + Send + Sync>> {
        self.source.downcast()
    }
    pub fn downcast_ref<E: std::error::Error + Send + Sync + 'static>(&self) -> Option<&E> {
        self.source.downcast_ref()
    }
    pub fn downcast_mut<E: std::error::Error + Send + Sync + 'static>(&mut self) -> Option<&mut E> {
        self.source.downcast_mut()
    }
}

pub trait WithContext<T, E, C> {
    type Err;
    fn context(self, context: C) -> Result<T, Self::Err>;
    fn context_with<F>(self, f: F) -> Result<T, Self::Err>
    where
        F: FnOnce(&E) -> C;
}
impl<T, E: IntoContext, C> WithContext<T, E, C> for Result<T, E> {
    type Err = Context<C>;
    fn context(self, context: C) -> Result<T, <Self as WithContext<T, E, C>>::Err> {
        self.context_with(|_| context)
    }
    fn context_with<F>(self, f: F) -> Result<T, <Self as WithContext<T, E, C>>::Err>
    where
        F: FnOnce(&E) -> C,
    {
        self.map_err(|e| {
            let context = f(&e);
            e.context(context)
        })
    }
}
impl<T, C: std::error::Error + Send + Sync + 'static> WithContext<T, (), C> for Option<T> {
    type Err = Wrap;
    fn context(self, context: C) -> Result<T, <Self as WithContext<T, (), C>>::Err> {
        self.context_with(|_| context)
    }
    fn context_with<F>(self, f: F) -> Result<T, <Self as WithContext<T, (), C>>::Err>
    where
        F: FnOnce(&()) -> C,
    {
        self.ok_or_else(|| f(&()).into())
    }
}

#[derive(Error, Debug, Clone, PartialEq, Eq)]
pub enum RunCommandError {
    #[error("at least one serde format is required")]
    UndefinedSerializeFormat,
    #[error("should be KEY=VALUE format, but `{0}` has no '='")]
    KeyValueFormat(String),
    #[error("`{0}` is unknown extension format")]
    UnknownFormatExtension(String),
    #[error("cannot read some configs")]
    CannotReadSomeConfigs(Vec<Config<HttpRequest, HttpResponse>>),
    #[error("cannot specify format")]
    CannotSpecifyFormat,
    #[error("nan is not number")]
    NanPercentile,
}

#[derive(Error, Debug, Clone, PartialEq, Eq)]
pub enum TemplateError {
    #[error("{0}")]
    NomParseError(String),
    #[error("remaining template: {0}")]
    RemainingTemplate(String),
    #[error("variable `{0}` is not defined")]
    VariableNotDefined(String),
}

#[derive(Error, Debug, Clone, PartialEq, Eq)]
pub enum AssaultError {
    #[error("cannot specify service")]
    CannotSpecifyService,
}

#[derive(Error, Debug, Clone, PartialEq, Eq)]
pub enum ReportError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_wrap() {
        fn wrap_any_error() -> WrappedResult<()> {
            Err(RunCommandError::CannotSpecifyFormat)?
        }

        assert_eq!(wrap_any_error().unwrap_err().downcast_ref(), Some(&RunCommandError::CannotSpecifyFormat));
    }

    #[test]
    fn test_nested_context() {
        fn nested_context() -> WrappedResult<()> {
            Err(RunCommandError::CannotSpecifyFormat).context(true).context("two").context(3)?
        }

        let err = nested_context().unwrap_err();
        let Context { context: 3, source } = err.downcast_ref().unwrap() else { panic!() };
        let Context { context: "two", source } = source.downcast_ref().unwrap() else { panic!() };
        let Context { context: true, source } = source.downcast_ref().unwrap() else { panic!() };
        assert_eq!(source.downcast_ref(), Some(&RunCommandError::CannotSpecifyFormat));
    }

    #[test]
    fn test_crate_error() {
        fn crate_error() -> crate::Result<()> {
            fn wrapped() -> WrappedResult<()> {
                Err(RunCommandError::CannotSpecifyFormat)?
            }
            Ok(wrapped()?)
        }

        assert_eq!(crate_error().unwrap_err().downcast_ref(), Some(&RunCommandError::CannotSpecifyFormat));
    }

    #[test]
    fn test_multi_wrap() {
        fn multi_wrap(n: usize) -> WrappedResult<()> {
            Err((0..n).map(|_| RunCommandError::CannotSpecifyFormat.into()).collect::<MultiWrap>())?
        }

        assert_eq!(multi_wrap(0).unwrap_err().to_string(), "");
        assert_eq!(
            multi_wrap(1).unwrap_err().to_string(),
            [format!("{}", RunCommandError::CannotSpecifyFormat)].join("\n")
        );
        assert_eq!(
            multi_wrap(2).unwrap_err().to_string(),
            [format!("{}", RunCommandError::CannotSpecifyFormat), format!("{}", RunCommandError::CannotSpecifyFormat),]
                .join("\n")
        );
        assert_eq!(
            multi_wrap(3).unwrap_err().to_string(),
            [
                format!("{}", RunCommandError::CannotSpecifyFormat),
                format!("{}", RunCommandError::CannotSpecifyFormat),
                format!("{}", RunCommandError::CannotSpecifyFormat),
            ]
            .join("\n")
        );
        assert_eq!(
            multi_wrap(4).unwrap_err().to_string(),
            [
                &format!("{}", RunCommandError::CannotSpecifyFormat),
                &format!("{}", RunCommandError::CannotSpecifyFormat),
                &format!("{}", RunCommandError::CannotSpecifyFormat),
                r#"... and 1 more"#,
            ]
            .join("\n")
        );
        assert_eq!(
            multi_wrap(5).unwrap_err().to_string(),
            [
                &format!("{}", RunCommandError::CannotSpecifyFormat),
                &format!("{}", RunCommandError::CannotSpecifyFormat),
                &format!("{}", RunCommandError::CannotSpecifyFormat),
                r#"... and 2 more"#,
            ]
            .join("\n")
        );
        assert_eq!(
            multi_wrap(100).unwrap_err().to_string(),
            [
                &format!("{}", RunCommandError::CannotSpecifyFormat),
                &format!("{}", RunCommandError::CannotSpecifyFormat),
                &format!("{}", RunCommandError::CannotSpecifyFormat),
                r#"... and 97 more"#,
            ]
            .join("\n")
        );
    }
}
