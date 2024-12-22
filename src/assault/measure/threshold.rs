use std::time::Duration;

use super::aggregate::{PassAggregate, ResponseAggregate};

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Classified {
    Good,
    Allow,
    Warn,
    Bad,
}

impl Classified {
    pub fn pass_agg(pass: &PassAggregate) -> Self {
        if pass.count == pass.pass {
            Self::Good
        } else if pass.pass_rate > 0.7 {
            Self::Allow
        } else if pass.pass_rate > 0.5 {
            Self::Warn
        } else {
            Self::Bad
        }
    }

    pub fn response_agg(_response: &ResponseAggregate) -> Self {
        Self::Good
    }

    pub fn latency(latency: Duration) -> Self {
        if latency > Duration::from_secs(3) {
            Self::Bad
        } else if latency > Duration::from_secs(1) {
            Self::Warn
        } else if latency > Duration::from_millis(200) {
            Self::Allow
        } else {
            Self::Good
        }
    }
}
