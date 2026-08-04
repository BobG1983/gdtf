//! Per-cell move cost grid with a default and optional overrides.

use bevy::{platform::collections::HashMap, prelude::Resource};

use crate::{metric::CellLevel, tuning::MoveCost};

/// Move cost for each cell; falls back to a default when no override is set.
#[derive(Resource, Debug, Clone, PartialEq, Eq)]
pub struct FloorCostGrid {
    default:   MoveCost,
    overrides: HashMap<CellLevel, MoveCost>,
}

impl FloorCostGrid {
    /// Build from a default cost and optional per-cell overrides.
    #[must_use]
    pub fn new(
        default: MoveCost,
        overrides: impl IntoIterator<Item = (CellLevel, MoveCost)>,
    ) -> Self {
        Self {
            default,
            overrides: overrides.into_iter().collect(),
        }
    }

    /// Cost to enter this cell.
    #[must_use]
    pub fn cost(&self, at: &CellLevel) -> MoveCost {
        self.overrides.get(at).copied().unwrap_or(self.default)
    }
}
