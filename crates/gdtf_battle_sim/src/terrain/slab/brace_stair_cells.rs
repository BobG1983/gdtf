//! Cells that are the lower endpoint of an authored stair and can grant terrain brace.

use bevy::{platform::collections::HashSet, prelude::Resource};

use crate::metric::CellLevel;

/// Set of stair lower-endpoint cells eligible for terrain brace.
#[derive(Resource, Debug, Clone, Default)]
pub struct BraceStairCells(HashSet<CellLevel>);

impl BraceStairCells {
    /// Build from an existing set.
    #[must_use]
    pub const fn new(cells: HashSet<CellLevel>) -> Self {
        Self(cells)
    }

    /// Empty set.
    #[must_use]
    pub fn empty() -> Self {
        Self::default()
    }

    /// Whether this cell is a brace-eligible stair endpoint.
    #[must_use]
    pub fn contains(&self, cell_level: &CellLevel) -> bool {
        self.0.contains(cell_level)
    }
}
