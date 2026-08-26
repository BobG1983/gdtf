//! Cells a ganger can walk to, each with what reaching it costs.

use serde::{Deserialize, Serialize};

use super::{cell::CellLevelNet, vitals::TuNet};

/// A cell the search reached, and the TU walking to it charges.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ReachableCellNet {
    /// Where the walk ends.
    pub at:   CellLevelNet,
    /// What reaching it charges.
    pub cost: TuNet,
}

impl ReachableCellNet {
    /// Build from a cell and the TU reaching it charges.
    #[must_use]
    pub const fn new(at: CellLevelNet, cost: TuNet) -> Self {
        Self { at, cost }
    }
}
