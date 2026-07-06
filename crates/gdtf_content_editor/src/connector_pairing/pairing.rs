//! The pairing commit + its outcome vocabulary (GTW-531 C2 / C4) — the entry point the prefab
//! viewport click-commit drives.

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

/// The outcome of an auto-paired placement (GTW-531 C2) — whether the requested placement landed and
/// whether its paired DOWN connector was also placed.
///
/// A named domain enum (no-bare-types: the pairing outcome is a domain value, not a bare `bool`
/// tuple). The caller (the prefab viewport click-commit) reads it to decide whether to redraw and
/// what to log; a test asserts the [`PairPlaced`](PairingOutcome::PairPlaced) case fires.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PairingOutcome {
    /// The requested placement was REJECTED by the shared legality predicate (out of bounds /
    /// slab-seals-ladder). Nothing was placed; there is nothing to pair.
    Rejected,
    /// The requested tile was placed, and it is NOT an up connector (or has no resolvable DOWN
    /// counterpart) — a plain single placement, no pair.
    PlacedNoPair,
    /// The requested tile was placed and IS an up connector, but its DOWN counterpart could NOT be
    /// placed above — either `N` is the top storey (fail-closed, no `N+1` in range) or the shared
    /// predicate rejected the pair placement at `N+1` (e.g. a conflict there). The up connector
    /// still landed.
    PlacedPairSkipped,
    /// The requested UP connector was placed at `(x, y, N)` AND its paired DOWN connector was placed
    /// at `(x, y, N+1)` — the headline GTW-531 behaviour.
    PairPlaced {
        /// The DOWN counterpart terrain that was auto-placed one storey up.
        down: TerrainUuid,
        /// The slot the DOWN counterpart was placed into (`(x, y, N+1)`).
        at:   CellLevel,
    },
}

impl PairingOutcome {
    /// Whether ANY terrain was written to the map by this call — the question the viewport's
    /// redraw-on-commit asks (a rejected placement changes nothing, so no redraw is needed).
    #[must_use]
    pub const fn changed_map(&self) -> bool {
        !matches!(self, Self::Rejected)
    }
}

/// Commit a placement through the shared [`apply_placement`] predicate AND auto-place the paired
/// DOWN connector one storey up when the placed tile is an UP connector (GTW-531 C2 / C4) — the
/// entry point the prefab viewport click-commit drives.
///
/// REUSES the shared GTW-430 legality VERBATIM for BOTH the requested placement and the pair
/// placement (never re-implemented — C6). Behaviour:
///
/// - The requested `placement` runs through [`apply_placement`] first (out-of-bounds /
///   slab-seals-ladder reject, ladder-auto-clear side-effect). If it is rejected the whole call is
///   [`Rejected`](PairingOutcome::Rejected) and the map is unchanged.
/// - If it landed and the tile is an up connector with a resolvable DOWN counterpart, the DOWN
///   counterpart is placed at `(x, y, N+1)` through [`apply_placement`] too. If `N+1` is out of the
///   prefab's level range (fail-closed at the top) OR the shared predicate rejects the pair, the
///   pair is SKIPPED ([`PlacedPairSkipped`](PairingOutcome::PlacedPairSkipped)) — the up connector
///   still landed; the pair placement never silently corrupts existing terrain (the shared conflict
///   rules govern it).
/// - Otherwise a plain [`PlacedNoPair`](PairingOutcome::PlacedNoPair).
///
/// One-way (C4): this does NOT link removal — removing an up connector never auto-removes its pair.
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
        // Placed, but not an up connector (or no counterpart) — a plain single placement.
        return PairingOutcome::PlacedNoPair;
    };

    // Fail-closed at the top: no storey above `N` inside the prefab's level range → place only the
    // up connector, skip the pair (never auto-place above the top).
    let Some(above) = level_above(placement.slot(), size) else {
        info!(
            "GTW-531: up connector placed on the top storey — paired DOWN connector skipped \
             (fail-closed, no storey above)"
        );
        return PairingOutcome::PlacedPairSkipped;
    };

    // Place the DOWN counterpart at (x, y, N+1) through the SAME shared predicate — its conflict
    // rules (slab-seals-ladder, out-of-bounds) govern the pair placement; a rejection there skips
    // the pair rather than corrupting existing terrain.
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

/// The slot one storey ABOVE `slot`, or [`None`] if `slot` is already at the prefab's top storey
/// (fail-closed for the pair placement — C2). Mirrors the placement module's own `level_above`, but
/// clamped to the PREFAB'S `size.levels()` (not just [`MAX_LEVELS`]) so the pair is never placed
/// past the authored volume.
#[must_use]
fn level_above(slot: CellLevel, size: GridSize) -> Option<CellLevel> {
    // The storey via the canonical `CellLevel::level` accessor (GTW-565), then a
    // checked u8 add against BOTH ceilings.
    let next = (*slot.level()).checked_add(1)?;
    if next >= MAX_LEVELS || next >= *size.levels() {
        return None;
    }
    Some(CellLevel::new(slot.cell(), Level::new(next)))
}
