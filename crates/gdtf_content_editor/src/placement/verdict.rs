//! The placement vocabulary (GTW-430) — the [`IllegalReason`] rejection reasons, the
//! [`PlacementVerdict`] the one shared predicate returns, and the [`ProposedPlacement`] value type
//! the hover-ghost and the click-commit both build.

use gdtf_battle_sim::{metric::CellLevel, terrain::def::TerrainUuid};

/// Why a placement is illegal — the reason a [`PlacementVerdict::Illegal`] carries (GTW-430 C2).
///
/// A named domain enum (no-bare-types: a rejection reason is a domain value). Each variant is one
/// of the editor's placement rules; the ghost shows the cell red and the commit rejects for ANY of
/// them.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IllegalReason {
    /// The target slot is outside the drawable volume (the [`EditorMap`](crate::editor_map::EditorMap) clamp — C3).
    OutOfBounds,
    /// A slab would seal an existing ladder — either at the SAME slot (a ladder rises through the
    /// cell) or directly BELOW (the ladder's destination). The single-plane canvas reaches the
    /// same-slot case (C2).
    SlabSealsLadder,
}

/// The verdict the SINGLE SHARED legality predicate returns (GTW-430 C3).
///
/// A named domain enum (no-bare-types: a legality verdict is a domain value, not a bare `bool` /
/// `Result`). [`Legal`](PlacementVerdict::Legal) carries any vertical SIDE-EFFECT the commit must
/// perform (the C1 auto-clear); [`Illegal`](PlacementVerdict::Illegal) carries the
/// [`IllegalReason`] the ghost tints red for and the commit rejects on. Both the hover-ghost
/// preview and the click-commit consume this same verdict — there is no duplicated rule logic.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PlacementVerdict {
    /// The placement is legal — the commit may write it. Carries the optional vertical side-effect
    /// the commit applies first (the C1 auto-clear of a slab above a ladder).
    Legal {
        /// The slot whose paint the placement AUTO-CLEARS (C1: the slab one storey above a placed
        /// ladder), or [`None`] when the placement has no vertical side-effect.
        auto_clear: Option<CellLevel>,
    },
    /// The placement is illegal — the ghost tints the target red and the commit rejects it.
    Illegal(IllegalReason),
}

impl PlacementVerdict {
    /// A legal placement with no vertical side-effect.
    #[must_use]
    pub const fn legal() -> Self {
        Self::Legal { auto_clear: None }
    }

    /// A legal placement that AUTO-CLEARS the paint at `slot` first (C1).
    #[must_use]
    pub const fn legal_clearing(slot: CellLevel) -> Self {
        Self::Legal {
            auto_clear: Some(slot),
        }
    }

    /// Whether this verdict is illegal — the question the ghost (tint red?) and the commit
    /// (reject?) both ask.
    #[must_use]
    pub const fn is_illegal(&self) -> bool {
        matches!(self, Self::Illegal(_))
    }
}

/// A proposed placement — the `(slot, tile)` the author is hovering / clicking (GTW-430).
///
/// A named struct (not a bare tuple) so the placement the legality predicate consumes is
/// self-describing: the [`CellLevel`] target slot (cell + storey) and the [`TerrainUuid`] of the
/// tile being placed. The hover-ghost builds one for the hovered cell + selected tile; the commit
/// builds the same for the clicked cell.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ProposedPlacement {
    /// The target slot (cell + storey) the tile would be painted into.
    slot: CellLevel,
    /// The tile being placed.
    tile: TerrainUuid,
}

impl ProposedPlacement {
    /// Build a proposed placement from its target slot and the terrain being placed.
    #[must_use]
    pub const fn new(slot: CellLevel, tile: TerrainUuid) -> Self {
        Self { slot, tile }
    }

    /// The target slot (cell + storey).
    #[must_use]
    pub const fn slot(&self) -> CellLevel {
        self.slot
    }

    /// The terrain being placed.
    #[must_use]
    pub const fn tile(&self) -> TerrainUuid {
        self.tile
    }
}
