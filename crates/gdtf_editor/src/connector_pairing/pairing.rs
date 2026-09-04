//! Apply placement, placing a staircase's second end one storey up with it.

use bevy::prelude::*;
use gdtf_battle_sim::{
    level::{GridSize, ThemeUuid},
    metric::{CellLevel, Level, MAX_LEVELS},
    terrain::def::{TerrainDefRegistry, TerrainUuid},
};

use super::is_stair;
use crate::{
    editor_map::EditorMap,
    placement::{ProposedPlacement, apply_placement},
};

/// Result of placing a tile that may auto-pair a staircase.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PairingOutcome {
    /// Placement rejected by rules.
    Rejected,
    /// Placed; the tile is not a staircase.
    PlacedNoPair,
    /// Staircase placed; the second end one storey up was skipped.
    PlacedPairSkipped,
    /// Staircase placed, and its second end placed one storey up.
    PairPlaced {
        /// The tile the pass placed one storey up, the same tile that was painted.
        paired: TerrainUuid,
        /// Slot where that second tile landed.
        at:     CellLevel,
    },
}

impl PairingOutcome {
    /// Whether the map was modified.
    #[must_use]
    pub const fn changed_map(&self) -> bool {
        !matches!(self, Self::Rejected)
    }
}

/// Place a tile and, if it is a staircase, try to place its second end one storey up.
pub fn apply_placement_with_pairing(
    map: &mut EditorMap,
    registry: &TerrainDefRegistry,
    theme: ThemeUuid,
    placement: &ProposedPlacement,
    size: GridSize,
) -> PairingOutcome {
    if !apply_placement(map, registry, theme, placement, size) {
        return PairingOutcome::Rejected;
    }

    let paired = placement.tile();
    if !is_stair(registry, &paired) {
        return PairingOutcome::PlacedNoPair;
    }

    let Some(above) = level_above(placement.slot(), size) else {
        info!(
            "staircase placed on the top storey — its second end was skipped (fail-closed, no \
             storey above)"
        );
        return PairingOutcome::PlacedPairSkipped;
    };

    let pair = ProposedPlacement::new(above, paired, placement.facing());
    if apply_placement(map, registry, theme, &pair, size) {
        PairingOutcome::PairPlaced { paired, at: above }
    } else {
        info!(
            "staircase placed, but its second end at the storey above was rejected by the \
             shared placement predicate (conflict) — pair skipped"
        );
        PairingOutcome::PlacedPairSkipped
    }
}

#[must_use]
fn level_above(slot: CellLevel, size: GridSize) -> Option<CellLevel> {
    let next = (*slot.level()).checked_add(1)?;
    if next >= MAX_LEVELS || next >= *size.levels() {
        return None;
    }
    Some(CellLevel::new(slot.cell(), Level::new(next)))
}
