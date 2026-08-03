//! Placement legality verdict and proposed placement.

use gdtf_battle_sim::{metric::CellLevel, terrain::def::TerrainUuid};

/// Why a placement was rejected.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IllegalReason {
    /// Slot is outside the grid.
    OutOfBounds,
    /// A slab would seal a ladder slot.
    SlabSealsLadder,
}

/// Result of evaluating a placement.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PlacementVerdict {
    /// Placement is allowed.
    Legal {
        /// Optional slot to clear first (e.g. ladder auto-clear).
        auto_clear: Option<CellLevel>,
    },
    /// Placement is not allowed.
    Illegal(IllegalReason),
}

impl PlacementVerdict {
    /// Legal with no auto-clear.
    #[must_use]
    pub const fn legal() -> Self {
        Self::Legal { auto_clear: None }
    }

    /// Legal and clear `slot` first.
    #[must_use]
    pub const fn legal_clearing(slot: CellLevel) -> Self {
        Self::Legal {
            auto_clear: Some(slot),
        }
    }

    /// Whether this verdict is illegal.
    #[must_use]
    pub const fn is_illegal(&self) -> bool {
        matches!(self, Self::Illegal(_))
    }
}

/// A tile the author wants to place at a slot.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ProposedPlacement {
    slot: CellLevel,
    tile: TerrainUuid,
}

impl ProposedPlacement {
    /// Build a placement proposal.
    #[must_use]
    pub const fn new(slot: CellLevel, tile: TerrainUuid) -> Self {
        Self { slot, tile }
    }

    /// Target slot.
    #[must_use]
    pub const fn slot(&self) -> CellLevel {
        self.slot
    }

    /// Tile to place.
    #[must_use]
    pub const fn tile(&self) -> TerrainUuid {
        self.tile
    }
}
