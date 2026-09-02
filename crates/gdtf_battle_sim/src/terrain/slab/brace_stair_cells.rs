//! Cells that are the lower endpoint of an authored stair and can grant terrain brace.

use bevy::{platform::collections::HashSet, prelude::Resource};

use crate::metric::{CellLevel, Level};

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

    /// Whether a slab at this cell rests on a brace-eligible stair endpoint.
    #[must_use]
    pub fn braces_slab_at(&self, at: &CellLevel) -> bool {
        cell_below(at).is_some_and(|below| self.contains(&below))
    }
}

// The cell one storey down, or nothing at the bottom level.
fn cell_below(at: &CellLevel) -> Option<CellLevel> {
    let storey = (*at.level()).checked_sub(1)?;
    Some(CellLevel::new(at.cell(), Level::new(storey)))
}
