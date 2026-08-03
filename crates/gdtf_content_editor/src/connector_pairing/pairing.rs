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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PairingOutcome {
            Rejected,
            PlacedNoPair,
                    PlacedPairSkipped,
            PairPlaced {
                down: TerrainUuid,
                at:   CellLevel,
    },
}

impl PairingOutcome {
            #[must_use]
    pub const fn changed_map(&self) -> bool {
        !matches!(self, Self::Rejected)
    }
}

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
            "GTW-531: up connector placed on the top storey — paired DOWN connector skipped \
             (fail-closed, no storey above)"
        );
        return PairingOutcome::PlacedPairSkipped;
    };

    let pair = ProposedPlacement::new(above, down);
    if apply_placement(map, registry, theme, &pair, size) {
        PairingOutcome::PairPlaced { down, at: above }
    } else {
        info!(
            "GTW-531: up connector placed, but the paired DOWN connector at the storey above was \
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
