//! Cross-storey badge kinds and the per-cell signal resource.

use bevy::{platform::collections::HashMap, prelude::*};
use gdtf_battle_sim::{falls::StoreysFallen, prelude::Cell};

/// Signed storey offset relative to the active level.
#[derive(Deref, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct LevelDelta(i8);

impl LevelDelta {
    /// Build from a signed storey delta.
    #[must_use]
    pub const fn new(delta: i8) -> Self {
        Self(delta)
    }

    /// `true` when the signal is above the active level.
    #[must_use]
    pub const fn is_above(self) -> bool {
        self.0 > 0
    }

    /// Absolute storey distance.
    #[must_use]
    pub const fn magnitude(self) -> u8 {
        self.0.unsigned_abs()
    }
}

/// Number of threats aggregated on one cell.
#[derive(Deref, Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct ThreatCount(u8);

impl ThreatCount {
    /// Single threat.
    pub const ONE: Self = Self(1);

    /// Add one threat, saturating at `u8::MAX`.
    #[must_use]
    pub const fn incremented(self) -> Self {
        Self(self.0.saturating_add(1))
    }
}

/// Kind of badge drawn for a cross-level signal.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CrossLevelBadgeKind {
    /// Enemy presence on another storey.
    Threat {
        /// Storey offset of the threat.
        delta: LevelDelta,
        /// How many threats share this cell.
        count: ThreatCount,
    },
    /// Fall depth marker.
    DropDepth {
        /// Storeys fallen.
        storeys: StoreysFallen,
    },
    /// Stair / ladder connector to another storey.
    ConnectorDelta {
        /// Storey offset of the connector destination.
        delta: LevelDelta,
    },
}

/// Max badges drawn on one cell.
pub const BADGE_CAP_PER_CELL: usize = 3;

/// Derived cross-level badges keyed by ground cell.
#[derive(Resource, Debug, Clone, Default, PartialEq, Eq)]
pub struct CrossLevelSignals {
    per_cell: HashMap<Cell, Vec<CrossLevelBadgeKind>>,
}

impl CrossLevelSignals {
    #[must_use]
    pub(super) const fn build(per_cell: HashMap<Cell, Vec<CrossLevelBadgeKind>>) -> Self {
        Self { per_cell }
    }

    /// Badges for a cell, empty when none.
    #[must_use]
    pub fn badges_at(&self, cell: Cell) -> &[CrossLevelBadgeKind] {
        self.per_cell.get(&cell).map_or(&[], Vec::as_slice)
    }

    /// Cells that have at least one badge.
    pub fn cells(&self) -> impl Iterator<Item = Cell> + '_ {
        self.per_cell.keys().copied()
    }
}
