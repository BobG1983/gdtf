//! map for cells that author a different floor piece (the `floors` list). The
use bevy::{platform::collections::HashMap, prelude::Resource};

use crate::{metric::CellLevel, tuning::MoveCost};

#[derive(Resource, Debug, Clone, PartialEq, Eq)]
pub struct FloorCostGrid {
        default:   MoveCost,
        overrides: HashMap<CellLevel, MoveCost>,
}

impl FloorCostGrid {
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

                        #[must_use]
    pub fn cost(&self, at: &CellLevel) -> MoveCost {
        self.overrides.get(at).copied().unwrap_or(self.default)
    }
}
