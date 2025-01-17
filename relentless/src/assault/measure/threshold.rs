use std::{
    ops::{Deref, DerefMut},
    time::Duration,
};

use crate::assault::{messages::Messages, reportable::Reportable};

use super::aggregate::{PassAggregate, ResponseAggregate};

// TODO better implementation ?
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Classification {
    Good,
    Allow,
    Warn,
    Bad,
}

pub trait Classify {
    fn classify(&self) -> Classification;
    fn classified(self) -> Classified<Self>
    where
        Self: Sized,
    {
        match self.classify() {
            Classification::Good => Classified(Classification::Good, self),
            Classification::Allow => Classified(Classification::Allow, self),
            Classification::Warn => Classified(Classification::Warn, self),
            Classification::Bad => Classified(Classification::Bad, self),
        }
    }
}
impl<T: Reportable> Classify for T {
    fn classify(&self) -> Classification {
        if self.pass() {
            Classification::Good
        } else {
            Classification::Bad
        }
    }
}
impl<M> Classify for Messages<M> {
    fn classify(&self) -> Classification {
        if self.is_empty() {
            Classification::Good
        } else {
            Classification::Warn
        }
    }
}
impl Classify for PassAggregate {
    fn classify(&self) -> Classification {
        if self.count == self.pass {
            Classification::Good
        } else if self.pass_rate > 0.8 {
            Classification::Allow
        } else if self.pass_rate > 0.5 {
            Classification::Warn
        } else {
            Classification::Bad
        }
    }
}
impl Classify for ResponseAggregate {
    fn classify(&self) -> Classification {
        Classification::Good
    }
}
impl Classify for Duration {
    fn classify(&self) -> Classification {
        if self > &Duration::from_secs(3) {
            Classification::Bad
        } else if self > &Duration::from_secs(1) {
            Classification::Warn
        } else if self > &Duration::from_millis(200) {
            Classification::Allow
        } else {
            Classification::Good
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Classified<T>(Classification, T);
impl<T> Deref for Classified<T> {
    type Target = T;
    fn deref(&self) -> &Self::Target {
        &self.1
    }
}
impl<T> DerefMut for Classified<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.1
    }
}
impl<T> Classified<T> {
    pub fn class(&self) -> Classification {
        self.0.clone()
    }
}
