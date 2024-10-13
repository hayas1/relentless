use std::{
    fmt::{Debug, Display},
    ops::{Deref, DerefMut},
};

use thiserror::Error;

use crate::config::Config;

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
impl RelentlessError {
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
        Self(Box::new(e))
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
}

#[derive(Debug)]
pub struct MultiWrap(pub Vec<Wrap>);
impl Display for MultiWrap {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // TODO implement ... and more n
        for wrap in &self.0 {
            write!(f, "{}", wrap)?;
        }
        Ok(())
    }
}
impl std::error::Error for MultiWrap {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        // TODO multiple sources ?
        self.0.first().map(|w| w.0.as_ref() as _)
    }
}
impl FromIterator<Wrap> for MultiWrap {
    fn from_iter<T: IntoIterator<Item = Wrap>>(iter: T) -> Self {
        Self(iter.into_iter().collect())
    }
}
impl Deref for MultiWrap {
    type Target = Vec<Wrap>;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}
impl DerefMut for MultiWrap {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

pub trait IntoContext: std::error::Error + Send + Sync + 'static + Sized {
    fn context<C>(self, context: C) -> Context<C> {
        Context { context, source: Box::new(self) }
    }
}
impl<E: std::error::Error + Send + Sync + 'static> IntoContext for E {}
#[derive(Debug)]
pub struct Context<C> {
    pub context: C,
    pub source: Box<dyn std::error::Error + Send + Sync>,
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
    pub fn context_ref(&self) -> &C {
        &self.context
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

#[derive(Error, Debug)]
pub enum RunCommandError {
    #[error("should be KEY=VALUE format, but `{0}` has no '='")]
    KeyValueFormat(String),
    #[error("`{0}` is unknown extension format")]
    UnknownFormatExtension(String),
    #[error("cannot read some configs")]
    CannotReadSomeConfigs(Vec<Config>),
    #[error("cannot specify format")]
    CannotSpecifyFormat,
}

#[derive(Error, Debug)]
pub enum AssaultError {}

#[derive(Error, Debug)]
pub enum EvaluateError {}

#[derive(Error, Debug)]
pub enum ReportError {}
