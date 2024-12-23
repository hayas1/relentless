use std::{
    ops::{Deref, DerefMut},
    time::Duration,
};

use super::aggregate::{PassAggregate, ResponseAggregate};

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Classified<T> {
    Good(T),
    Allow(T),
    Warn(T),
    Bad(T),
}
impl<T> Deref for Classified<T> {
    type Target = T;
    fn deref(&self) -> &Self::Target {
        match self {
            Classified::Good(t) => t,
            Classified::Allow(t) => t,
            Classified::Warn(t) => t,
            Classified::Bad(t) => t,
        }
    }
}
impl<T> DerefMut for Classified<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        match self {
            Classified::Good(t) => t,
            Classified::Allow(t) => t,
            Classified::Warn(t) => t,
            Classified::Bad(t) => t,
        }
    }
}
pub trait Classify {
    fn classify(&self) -> Classified<()>;
    fn classified(self) -> Classified<Self>
    where
        Self: Sized,
    {
        match self.classify() {
            Classified::Good(()) => Classified::Good(self),
            Classified::Allow(()) => Classified::Allow(self),
            Classified::Warn(()) => Classified::Warn(self),
            Classified::Bad(()) => Classified::Bad(self),
        }
    }
}
impl Classify for PassAggregate {
    fn classify(&self) -> Classified<()> {
        if self.count == self.pass {
            Classified::Good(())
        } else if self.pass_rate > 0.8 {
            Classified::Allow(())
        } else if self.pass_rate > 0.5 {
            Classified::Warn(())
        } else {
            Classified::Bad(())
        }
    }
}
impl Classify for ResponseAggregate {
    fn classify(&self) -> Classified<()> {
        Classified::Good(())
    }
}
impl Classify for Duration {
    fn classify(&self) -> Classified<()> {
        if self > &Duration::from_secs(3) {
            Classified::Bad(())
        } else if self > &Duration::from_secs(1) {
            Classified::Warn(())
        } else if self > &Duration::from_millis(200) {
            Classified::Allow(())
        } else {
            Classified::Good(())
        }
    }
}
