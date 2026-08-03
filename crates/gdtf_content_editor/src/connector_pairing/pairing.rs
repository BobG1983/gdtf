//! Apply placement with automatic up/down connector pairing.

use bevy::prelude::*;
use gdtf_battle_sim::{
    level::{GridSize, ThemeUuid},
    metric::{CellLevel, Level, MAX_LEVELS},
    terrain::def::{TerrainDefRegistry, TerrainUuid},
};

use super::resolve_down_counterpart;
use crate::{
    editor_map::EditorMap,
    placement::{ProposedPlacement, apply_placement},
};

/// Result of placing a tile that may auto-pair a connector.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PairingOutcome {
    /// Placement rejected by rules.
    Rejected,
    /// Placed; tile is not an up connector.
    PlacedNoPair,
    /// Up connector placed; paired down was skipped.
    PlacedPairSkipped,
    /// Up connector placed and paired down placed above.
    PairPlaced {
        /// Down connector tile key.
        down: TerrainUuid,
        /// Slot where the down connector was placed.
        at: CellLevel,
    },
}

impl PairingOutcome {
    /// Whether the map was modified.
    #[must_use]
    pub const fn changed_map(&self) -> bool {
        !matches!(self, Self::Rejected)
    }
}

/// Place a tile and, if it is an up connector, try to place its down pair above.
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

    let Some(down) = resolve_down_counterpart(registry, &placement.tile()) else {
        return PairingOutcome::PlacedNoPair;
    };

    let Some(above) = level_above(placement.slot(), size) else {
        info!(
            "up connector placed on the top storey — paired DOWN connector skipped \
             (fail-closed, no storey above)"
        );
        return PairingOutcome::PlacedPairSkipped;
    };

    let pair = ProposedPlacement::new(above, down);
    if apply_placement(map, registry, theme, &pair, size) {
        PairingOutcome::PairPlaced { down, at: above }
    } else {
        info!(
            "up connector placed, but the paired DOWN connector at the storey above was \
             rejected by the shared placement predicate (conflict) — pair skipped"
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
