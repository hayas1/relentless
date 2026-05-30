use std::{fmt::Display, time::Duration};

use semigroup::Semigroup;
use serde::{Deserialize, Serialize};

use crate::{error::EvaluateError, shot::destinations::Destinations};

// TODO error handling
pub trait Evaluator<S: ?Sized> {
    type Message;
    fn evaluate_shot(&self, msg: &mut Messages<Self::Message>, res: &S) -> Result<(), Failure>;
    fn evaluate_compare(&self, msg: &mut Messages<Self::Message>, res1: &S, res2: &S) -> Result<(), Failure>;
    fn evaluate(&self, msg: &mut Messages<Self::Message>, res: Destinations<S>) -> Result<(), Failure>
    where
        Self::Message: From<EvaluateError>,
        S: Sized,
    {
        match res.len() {
            0 => Err(EvaluateError::EmptyTarget).map_err(|e| msg.error(e.into()))?,
            1 => self.evaluate_shots(msg, res),
            _ => self.evaluate_compares(msg, res),
        }
    }

    fn evaluate_bool<F: Fn(bool) -> Self::Message>(
        &self,
        msg: &mut Messages<Self::Message>,
        judge: bool,
        e: F,
    ) -> Result<(), Failure> {
        if judge {
            Ok(())
        } else {
            Err(msg.error(e(judge)))
        }
    }
    fn evaluate_shots(&self, msg: &mut Messages<Self::Message>, res: Destinations<S>) -> Result<(), Failure>
    where
        Self::Message: From<EvaluateError>,
        S: Sized,
    {
        let (_, resp) = res.into_iter().next().ok_or(EvaluateError::EmptyTarget).map_err(|e| msg.error(e.into()))?;
        self.evaluate_shot(msg, &resp)
    }
    fn evaluate_compares(&self, msg: &mut Messages<Self::Message>, res: Destinations<S>) -> Result<(), Failure>
    where
        Self::Message: From<EvaluateError>,
        S: Sized,
    {
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
        write!(f, "failure")
    }
}

pub trait MessageExt {
    fn timeout(time: Duration) -> Self;
}
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Default, Hash, Serialize, Deserialize)]
pub struct Message<M> {
    pub message: M,
    pub kind: MessageKind,
}
impl<M: Display> Display for Message<M> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.message)
    }
}
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Default, Hash, Serialize, Deserialize)]
pub enum MessageKind {
    #[default]
    Warn,
    Error,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Default, Hash, Serialize, Deserialize, Semigroup)]
#[semigroup(monoid, commutative, identity = "Messages::empty()", with = "semigroup::op::Concat")]
pub struct Messages<T>(Vec<Message<T>>);
impl<T> Messages<T> {
    pub fn new() -> Self {
        Self(Vec::new())
    }
    pub fn push(&mut self, message: Message<T>) {
        self.0.push(message);
    }
    pub fn warn(&mut self, message: T) {
        self.push(Message { message, kind: MessageKind::Warn });
    }
    pub fn error(&mut self, message: T) -> Failure {
        self.push(Message { message, kind: MessageKind::Error });
        Failure
    }
}

impl<T> Messages<T> {
    pub fn empty() -> Self {
        Self(Vec::new())
    }
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }
    pub fn len(&self) -> usize {
        self.0.len()
    }
    pub fn display_lines<'a>(&'a self) -> (impl 'a + Iterator<Item = &'a Message<T>>, Option<usize>) {
        let (n, m) = (self.0.len(), 3);
        let iter = self.0.iter().take(m);
        let more = (n > m).then(|| n - m);
        (iter, more)
    }
}
impl<T: Display> Display for Messages<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let (mut lines, and_more) = self.display_lines();
        lines.try_for_each(|l| writeln!(f, "{l}"))?;
        and_more.iter().try_for_each(|m| writeln!(f, "... and {m} more"))
    }
}
impl<T> Extend<Message<T>> for Messages<T> {
    fn extend<I: IntoIterator<Item = Message<T>>>(&mut self, iter: I) {
        self.0.extend(iter)
    }
}
