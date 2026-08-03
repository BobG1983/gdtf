//! Placement legality rules and apply helper.

use gdtf_battle_sim::{
    level::{GridSize, ThemeUuid},
    metric::{CellLevel, Level},
    terrain::def::TerrainDefRegistry,
};

use super::{EditorTileClass, IllegalReason, PlacementVerdict, ProposedPlacement, classify};
use crate::editor_map::EditorMap;

#[must_use]
fn is_ladder(
    map: &EditorMap,
    registry: &TerrainDefRegistry,
    theme: ThemeUuid,
    slot: CellLevel,
) -> bool {
    map.tile_at_level(slot)
        .is_some_and(|key| classify(registry, theme, &key) == EditorTileClass::Ladder)
}

#[must_use]
fn is_slab(
    map: &EditorMap,
    registry: &TerrainDefRegistry,
    theme: ThemeUuid,
    slot: CellLevel,
) -> bool {
    map.tile_at_level(slot)
        .is_some_and(|key| classify(registry, theme, &key) == EditorTileClass::Slab)
}

#[must_use]
fn level_above(slot: CellLevel) -> Option<CellLevel> {
    let next = (*slot.level()).checked_add(1)?;
    if next >= gdtf_battle_sim::metric::MAX_LEVELS {
        return None;
    }
    Some(CellLevel::new(slot.cell(), Level::new(next)))
}

/// Evaluate whether a placement is legal (and any auto-clear).
#[must_use]
pub fn evaluate_placement(
    map: &EditorMap,
    registry: &TerrainDefRegistry,
    theme: ThemeUuid,
    placement: &ProposedPlacement,
    size: GridSize,
) -> PlacementVerdict {
    let slot = placement.slot();
    if !slot_in_bounds(slot, size) {
        return PlacementVerdict::Illegal(IllegalReason::OutOfBounds);
    }
    let class = classify(registry, theme, &placement.tile());
    match class {
        EditorTileClass::Ladder => {
            if let Some(above) = level_above(slot)
                && is_slab(map, registry, theme, above)
            {
                return PlacementVerdict::legal_clearing(above);
            }
            PlacementVerdict::legal()
        }
        EditorTileClass::Slab => {
            let seals_ladder = is_ladder(map, registry, theme, slot)
                || level_below(slot).is_some_and(|below| is_ladder(map, registry, theme, below));
            if seals_ladder {
                return PlacementVerdict::Illegal(IllegalReason::SlabSealsLadder);
            }
            PlacementVerdict::legal()
        }
        EditorTileClass::Other => PlacementVerdict::legal(),
    }
}

#[must_use]
fn level_below(slot: CellLevel) -> Option<CellLevel> {
    let below = (*slot.level()).checked_sub(1)?;
    Some(CellLevel::new(slot.cell(), Level::new(below)))
}

fn slot_in_bounds(slot: CellLevel, size: GridSize) -> bool {
    let width = i32::from(*size.width());
    let height = i32::from(*size.height());
    let levels = i32::from(*size.levels());
    slot.x >= 0
        && slot.x < width
        && slot.y >= 0
        && slot.y < height
        && slot.z >= 0
        && slot.z < levels
}

/// Apply a legal placement (and optional auto-clear). Returns whether the map changed.
pub fn apply_placement(
    map: &mut EditorMap,
    registry: &TerrainDefRegistry,
    theme: ThemeUuid,
    placement: &ProposedPlacement,
    size: GridSize,
) -> bool {
    match evaluate_placement(map, registry, theme, placement, size) {
        PlacementVerdict::Illegal(_) => false,
        PlacementVerdict::Legal { auto_clear } => {
            if let Some(slot) = auto_clear {
                map.clear(slot);
            }
            map.paint_at(placement.slot(), placement.tile(), size)
        }
    }
}
