use std::fmt::Display;

use semigroup::Semigroup;

use crate::{error::EvaluateError, shot::destinations::Destinations};

// TODO error handling
pub trait Evaluator<S> {
    type Message;
    fn evaluate_shot(&self, msg: &mut Messages<Message<Self::Message>>, res: &S) -> Result<(), Failure>;
    fn evaluate_compare(&self, msg: &mut Messages<Message<Self::Message>>, res1: &S, res2: &S) -> Result<(), Failure>;

    fn evaluate<F: Fn(bool) -> Self::Message>(
        &self,
        msg: &mut Messages<Message<Self::Message>>,
        judge: bool,
        e: F,
    ) -> Result<(), Failure> {
        if judge {
            Ok(())
        } else {
            Err(msg.error(e(judge)))
        }
    }
    fn evaluate_shots(&self, msg: &mut Messages<Message<Self::Message>>, res: Destinations<S>) -> Result<(), Failure>
    where
        Self::Message: From<EvaluateError>,
    {
        let mut popper = res.into_iter();
        let (_, resp) = popper.next().ok_or(EvaluateError::EmptyTarget).map_err(|e| msg.error(e.into()))?;
        match popper.next() {
            Some(_) => Err(msg.error(EvaluateError::ShouldCompare.into())),
            None => self.evaluate_shot(msg, &resp),
        }
    }
    fn evaluate_compares(&self, msg: &mut Messages<Message<Self::Message>>, res: Destinations<S>) -> Result<(), Failure>
    where
        Self::Message: From<EvaluateError>,
    {
        match res.len() {
            0 => Err(EvaluateError::EmptyTarget).map_err(|e| msg.error(e.into()))?,
            1 => Err(EvaluateError::ShouldShot).map_err(|e| msg.error(e.into()))?,
            _ => (),
        }
        let v: Vec<_> = res.into_iter().collect();
        v.windows(2).try_fold((), |(), w| {
            let ((_, a), (_, b)) = (&w[0], &w[1]);
            self.evaluate_compare(msg, a, b)
        })
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

    pub fn displayable(self) -> Messages<String>
    where
        M: Display,
    {
        self.map(|m| m.message.to_string())
    }
}

impl<T> Messages<T> {
    pub fn map<U>(self, f: impl FnMut(T) -> U) -> Messages<U> {
        Messages(self.0.into_iter().map(f).collect())
    }
    pub fn empty() -> Self {
        Self(Vec::new())
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
