use clap::ValueEnum;
use serde::{Deserialize, Serialize};

#[cfg_attr(feature = "cli", derive(ValueEnum))]
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum Hierarchy {
    /// each job
    Job,

    /// each suite
    Suite,

    /// each testcases
    Testcase,
}

impl Hierarchy {
    pub fn contains(&self, hierarchies: &[Hierarchy]) -> bool {
        hierarchies.contains(self)
    }
}
