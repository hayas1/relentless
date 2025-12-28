use std::fmt::Display;

use semigroup::Semigroup;

use crate::{error::EvaluateError, shot::destinations::Destinations};

// TODO error handling
pub trait Evaluator<S> {
    type Warn;
    type Error;
    fn evaluate_shot(&self, res: &S) -> Result<Messages<Self::Warn>, Messages<Self::Error>>;
    fn evaluate_compare(&self, res1: &S, res2: &S) -> Result<Messages<Self::Warn>, Messages<Self::Error>>;

    fn evaluate<F: Fn(bool) -> Self::Error>(
        &self,
        judge: bool,
        e: F,
    ) -> Result<Messages<Self::Warn>, Messages<Self::Error>> {
        if judge {
            Ok(Messages::empty())
        } else {
            Err(e(judge))?
        }
    }
    fn evaluate_shots(&self, res: Destinations<S>) -> Result<Messages<Self::Warn>, Messages<Self::Error>>
    where
        Self::Error: From<EvaluateError>,
    {
        let mut popper = res.into_iter();
        let (_, resp) = popper.next().ok_or(EvaluateError::EmptyTarget).map_err(|e| Messages::one(e.into()))?;
        match popper.next() {
            Some(_) => Err(EvaluateError::ShouldCompare).map_err(|e| Messages::one(e.into()))?,
            None => self.evaluate_shot(&resp),
        }
    }
    fn evaluate_compares(&self, res: Destinations<S>) -> Result<Messages<Self::Warn>, Messages<Self::Error>>
    where
        Self::Error: From<EvaluateError>,
    {
        match res.len() {
            0 => Err(EvaluateError::EmptyTarget).map_err(|e| Messages::one(e.into()))?,
            1 => Err(EvaluateError::ShouldShot).map_err(|e| Messages::one(e.into()))?,
            _ => (),
        }
        let v: Vec<_> = res.into_iter().collect();
        // TODO try_combine
        let ok = v.windows(2).try_fold(Messages::empty(), |warn, w| {
            let ((_, a), (_, b)) = (&w[0], &w[1]);
            Ok(warn.semigroup(self.evaluate_compare(a, b)?))
        });
        ok
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Default, Hash)]
pub struct Failure;
impl std::error::Error for Failure {}
impl Display for Failure {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "fail")
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Default, Hash)]
pub struct Message<M> {
    pub message: M,
    pub kind: MessageKind,
}
impl<M: Display> Display for Message<M> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.message)
    }
}
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Default, Hash)]
pub enum MessageKind {
    #[default]
    Warn,
    Error,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Default, Hash, Semigroup)]
#[semigroup(monoid, commutative, identity = "Messages::empty()", with = "semigroup::op::Concat")]
// TODO size limit ? but causes it to no longer satisfy the properties of a semigroup, however strictly speaking, it already fails to satisfy the commutative property.
pub struct Messages<T>(Vec<T>);
impl<M> Messages<Message<M>> {
    pub fn new() -> Self {
        Self(Vec::new())
    }

    pub fn warn(&mut self, message: M) {
        self.0.push(Message { message, kind: MessageKind::Warn });
    }
    pub fn error(&mut self, message: M) -> Failure {
        self.0.push(Message { message, kind: MessageKind::Error });
        Failure
    }

    pub fn warn_result(&mut self, message: M) -> Result<(), Failure> {
        self.warn(message);
        Ok(())
    }
    pub fn error_result(&mut self, message: M) -> Result<(), Failure> {
        Err(self.error(message))
    }
    pub fn displayable(self) -> Messages<String>
    where
        M: Display,
    {
        self.map(|m| m.message.to_string())
    }
}

impl Messages<String> {
    pub fn flatten_display<W: Display, E: Display>(evaluated: &Result<Messages<W>, Messages<E>>) -> Self {
        match evaluated {
            Ok(m) => m.as_ref().map(|m| m.to_string()),
            Err(m) => m.as_ref().map(|m| m.to_string()),
        }
    }
}
impl<T> Messages<T> {
    pub fn flatten<W: Into<T>, E: Into<T>>(evaluated: Result<Messages<W>, Messages<E>>) -> Self {
        match evaluated {
            Ok(m) => m.map(Into::into),
            Err(m) => m.map(Into::into),
        }
    }
    pub fn map<U>(self, f: impl FnMut(T) -> U) -> Messages<U> {
        Messages(self.0.into_iter().map(f).collect())
    }
    pub fn empty() -> Self {
        Self(Vec::new())
    }
    pub fn one(msg: T) -> Self {
        Self(vec![msg])
    }
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }
    pub fn len(&self) -> usize {
        self.0.len()
    }
    pub fn display_lines<'a>(&'a self) -> (impl 'a + Iterator<Item = &'a T>, Option<usize>) {
        let (n, m) = (self.0.len(), 3);
        let iter = self.0.iter().take(m);
        let more = (n > m).then(|| n - m);
        (iter, more)
    }
    pub fn as_ref(&self) -> Messages<&T> {
        Messages(self.0.iter().collect())
    }
}
impl<T: std::error::Error> std::error::Error for Messages<T> {} // TODO multiple sources ?
impl<T> From<T> for Messages<T> {
    fn from(t: T) -> Self {
        Self::one(t)
    }
}
impl<T: Display> Display for Messages<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let (lines, and_more) = self.display_lines();
        for line in lines {
            writeln!(f, "{line}")?;
        }
        if let Some(num) = and_more {
            writeln!(f, "... and {num} more")?;
        }
        Ok(())
    }
}
impl<T> IntoIterator for Messages<T> {
    type Item = T;
    type IntoIter = <Vec<T> as IntoIterator>::IntoIter;
    fn into_iter(self) -> Self::IntoIter {
        self.0.into_iter()
    }
}
impl<T> Extend<T> for Messages<T> {
    fn extend<I: IntoIterator<Item = T>>(&mut self, iter: I) {
        self.0.extend(iter)
    }
}
