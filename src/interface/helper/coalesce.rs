use serde::{Deserialize, Serialize};

// TODO derive
pub trait Coalesce<O = Self> {
    fn coalesce(self, other: &O) -> Self;
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct Coalesced<T, O> {
    base: T,
    coalesced: Vec<O>,
}
impl<T: Clone + Coalesce<O>, O> Coalesced<T, O> {
    pub fn new(base: T, coalesced: Vec<O>) -> Self {
        Self { base, coalesced }
    }
    pub fn tuple(base: T, other: O) -> Self {
        Self::new(base, vec![other])
    }
    pub fn coalesce(&self) -> T {
        self.coalesced.iter().fold(self.base.clone(), |acc, x| acc.coalesce(x))
    }
    pub fn base(&self) -> &T {
        &self.base
    }
}
