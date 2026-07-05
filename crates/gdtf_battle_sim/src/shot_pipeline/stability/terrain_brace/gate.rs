//! The GTW-392 terrain-brace gate logic — [`terrain_braces`], its [`TerrainBraced`]
//! decision newtype, and the overhead-slab geometry helper [`cell_above`].

use bevy::prelude::Deref;

use crate::{
    ganger::{Position, StanceKind},
    metric::{CellLevel, Level, MAX_LEVELS},
    slab::BraceStairCells,
    surface::{SlabState, SurfaceGrid},
};

/// Whether a shooter earns the **terrain brace** — kneeling on a brace-eligible
/// (lower-endpoint) stair cell with an intact ([`SlabState::Present`]) slab directly
/// overhead (GTW-392, `docs/combat/resolution.md` §1a terrain-brace clause).
///
/// Geometry: a [`StanceKind::Crouching`] shooter on a brace-eligible stair cell `C`
/// braces when the slab at `(C.x, C.y, C.z + 1)` is `Present`. `Standing`/`Prone`
/// never brace (C3 — and a `Prone` occupant is single-cell on a stair anyway:
/// `register_stair_presence` skips the Low band, so the dual-cell stair relationship
/// a brace assumes is absent for prone). A destroyed or absent overhead slab never
/// braces (C4); a non-brace-cell / upper-endpoint cell never braces. Reads only the
/// change-driven world grids — no rebuild.
///
/// Returns `TerrainBraced::new(false)` (the fail-closed default) in every
/// non-brace case (wrong stance, out-of-brace-set, no overhead slab, top storey).
#[must_use]
pub fn terrain_braces(
    position: Position,
    stance: StanceKind,
    brace_cells: &BraceStairCells,
    surface: &SurfaceGrid,
) -> TerrainBraced {
    // C3: only Crouching earns the stair brace.
    if stance != StanceKind::Crouching {
        return TerrainBraced::new(false);
    }
    // Lower-endpoint filter: the brace-stair-cell set uses the same lower-endpoint
    // rule as the setup-time TerrainBrace marker placement — one set, one rule.
    let cell = *position;
    if !brace_cells.contains(&cell) {
        return TerrainBraced::new(false);
    }
    // Top-storey: no overhead slab can exist beyond MAX_LEVELS.
    let Some(above) = cell_above(cell) else {
        return TerrainBraced::new(false);
    };
    // C4: a Destroyed or Absent overhead slab revokes the brace at fire time.
    // The `SurfaceGrid::slab_state` read is the AUTHORITATIVE present/destroyed gate.
    TerrainBraced::new(matches!(surface.slab_state(&above), SlabState::Present))
}

/// The `(x, y, z + 1)` cell directly above `cell`, or `None` when `cell` is on the
/// top storey (`z >= MAX_LEVELS - 1`) — the overhead-slab geometry for the terrain-brace
/// gate.
///
/// `MAX_LEVELS` is `8` (`docs/combat/battle-space.md`: "60×60×8 coarse grid"); a slab
/// at storey 8 would be out of range, so a top-storey stair cell has no overhead slab.
#[must_use]
pub(crate) fn cell_above(cell: CellLevel) -> Option<CellLevel> {
    // The storey via the canonical CellLevel::level accessor (GTW-565), then a
    // checked u8 add against the storey ceiling — same x/y (the slab sits directly
    // above), storey + 1.
    let storey = (*cell.level()).checked_add(1)?;
    if storey >= MAX_LEVELS {
        return None;
    }
    Some(CellLevel::new(cell.cell(), Level::new(storey)))
}

/// Whether a shooter's **terrain-brace** engages — `true` iff a kneeling occupant on
/// the lower stair endpoint has an intact slab directly overhead (GTW-392).
///
/// A named newtype over `bool` (no-bare-types: a terrain-brace decision is a domain
/// value, not a bare boolean). Private inner + derived [`Deref`] (house style, matching
/// [`crate::weapon::Stable`] / [`crate::terrain::occupancy::DestroyedCover`]). `true`
/// means the terrain brace engages; `false` is the fail-closed default.
#[derive(Deref, Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct TerrainBraced(bool);

impl TerrainBraced {
    /// Build a terrain-brace decision from its boolean state — `true` when the
    /// stair-brace engages, `false` (the fail-closed default) otherwise.
    #[must_use]
    pub const fn new(braced: bool) -> Self {
        Self(braced)
    }
}
