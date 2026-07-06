//! The C1/C2 legality predicate + commit (GTW-430 C3) — [`evaluate_placement`] is the ONE shared
//! rule both the hover-ghost preview and the click-commit run; [`apply_placement`] is the commit
//! path that drives it.

use gdtf_battle_sim::{
    level::{GridSize, ThemeUuid},
    metric::{CellLevel, Level},
    terrain::def::TerrainDefRegistry,
};

use super::{EditorTileClass, IllegalReason, PlacementVerdict, ProposedPlacement, classify};
use crate::editor_map::EditorMap;

/// Whether the slot currently holds a tile that classifies as a LADDER (GTW-430).
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

/// Whether the slot currently holds a tile that classifies as a SLAB (GTW-430).
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

/// The slot one storey ABOVE `slot`, or [`None`] if `slot` is already at the storey ceiling
/// ([`MAX_LEVELS`](gdtf_battle_sim::metric::MAX_LEVELS) − 1).
///
/// The vertical rules reason about a cell's neighbour one level up (a ladder's destination / the
/// slab that would seal it). Saturating at the ceiling so a ladder on the top storey simply has no
/// slab-above to consider (and a placement there is never illegal for that reason).
#[must_use]
fn level_above(slot: CellLevel) -> Option<CellLevel> {
    // The storey via the canonical `CellLevel::level` accessor (GTW-565); rebuild the
    // slot one storey up via the typed constructor (never the raw IVec3).
    let next = (*slot.level()).checked_add(1)?;
    if next >= gdtf_battle_sim::metric::MAX_LEVELS {
        return None;
    }
    Some(CellLevel::new(slot.cell(), Level::new(next)))
}

/// The SINGLE SHARED placement-legality predicate (GTW-430 C3) — the one function the hover-ghost
/// preview and the click-commit both call.
///
/// Given the current [`EditorMap`], the [`TerrainDefRegistry`], the active theme, and a
/// [`ProposedPlacement`], returns the [`PlacementVerdict`]:
///
/// - **Out of bounds** → [`Illegal`](PlacementVerdict::Illegal) ([`IllegalReason::OutOfBounds`]).
/// - **Slab sealing an existing ladder** → [`Illegal`](PlacementVerdict::Illegal)
///   ([`IllegalReason::SlabSealsLadder`]).
/// - **Ladder with a slab directly above** → [`Legal`](PlacementVerdict::Legal) carrying the
///   `auto_clear` of that slab's slot (the C1 auto-handling).
/// - otherwise → a plain [`Legal`](PlacementVerdict::legal).
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
            // C1: a ladder's destination is the storey above. A slab there would seal it — auto-
            // clear it so the just-placed ladder is coherent (legal, with a side-effect).
            if let Some(above) = level_above(slot)
                && is_slab(map, registry, theme, above)
            {
                return PlacementVerdict::legal_clearing(above);
            }
            PlacementVerdict::legal()
        }
        EditorTileClass::Slab => {
            // C2: a slab is illegal where it would seal a ladder — at the SAME slot (a ladder rises
            // through the cell, reachable on the L0 canvas) or directly BELOW (the ladder's
            // destination). Reject it (the ghost tints the target red, the commit refuses).
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

/// The slot one storey BELOW `slot`, or [`None`] if `slot` is already the ground storey (`L0`).
///
/// The C2 slab rule checks the cell directly below for an existing ladder.
#[must_use]
fn level_below(slot: CellLevel) -> Option<CellLevel> {
    // The storey via the canonical `CellLevel::level` accessor (GTW-565); a ground
    // (L0) slot has nothing below (checked_sub is None).
    let below = (*slot.level()).checked_sub(1)?;
    Some(CellLevel::new(slot.cell(), Level::new(below)))
}

/// Whether `slot` falls inside the drawable volume — the legality predicate's bounds check, kept in
/// sync with the [`EditorMap`] clamp so the verdict and the model agree (C3).
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

/// Commit a proposed placement through the SHARED legality predicate (GTW-430 C2 + C1) — the path
/// the click-commit drives.
///
/// Calls [`evaluate_placement`]; on an [`Illegal`](PlacementVerdict::Illegal) verdict it REJECTS
/// (returns `false`, leaving the [`EditorMap`] UNCHANGED — C2). On a [`Legal`](PlacementVerdict::Legal)
/// verdict it first performs any vertical side-effect (the C1 auto-clear of a slab above a placed
/// ladder), then paints the tile and returns `true`. Returns whether the paint landed so the caller
/// redraws only on a committed paint.
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
            // C1: clear the slab the ladder would otherwise be sealed by, before painting.
            if let Some(slot) = auto_clear {
                map.clear(slot);
            }
            map.paint_at(placement.slot(), placement.tile(), size)
        }
    }
}
