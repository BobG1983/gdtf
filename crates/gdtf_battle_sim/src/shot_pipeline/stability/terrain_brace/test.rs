use bevy::platform::collections::HashSet;

use super::*;
use crate::{
    ganger::StanceKind,
    metric::{Cell, CellLevel, Level},
    slab::BraceStairCells,
    surface::SurfaceGrid,
};

/// Build a `(x, y, z)` `CellLevel` terse helper.
fn cl(x: i32, y: i32, z: u8) -> CellLevel {
    CellLevel::new(Cell::new(x, y), Level::new(z))
}

/// Build a [`BraceStairCells`] from a list of cells (the lower-endpoint set).
fn brace_cells(cells: &[CellLevel]) -> BraceStairCells {
    BraceStairCells::new(cells.iter().copied().collect::<HashSet<_>>())
}

/// Build a [`SurfaceGrid`] with `Present` slabs at the given cells.
fn surface_with_slabs(present: &[CellLevel]) -> SurfaceGrid {
    let mut grid = SurfaceGrid::new();
    for &cell in present {
        grid.set_slab(cell, SlabState::Present);
    }
    grid
}

// ── C1/C2 — kneeling on brace stair under Present slab braces ───────────

/// A [`StanceKind::Crouching`] shooter on a lower-endpoint stair cell with a
/// [`SlabState::Present`] slab directly overhead earns the terrain brace.
#[test]
fn kneeling_on_brace_stair_under_present_slab_braces() {
    let stair = cl(3, 4, 0);
    let overhead_slab = cl(3, 4, 1); // stair.z + 1
    let brace = brace_cells(&[stair]);
    let surface = surface_with_slabs(&[overhead_slab]);
    let pos = Position::new(stair);

    let result = terrain_braces(pos, StanceKind::Crouching, &brace, &surface);
    assert!(
        *result,
        "kneeling on a brace stair under a Present slab must earn the terrain brace",
    );
}

// ── C3 — stance gate ────────────────────────────────────────────────────

/// A [`StanceKind::Standing`] shooter on the same stair / slab geometry does NOT
/// earn the terrain brace (C3).
#[test]
fn standing_does_not_brace() {
    let stair = cl(3, 4, 0);
    let overhead_slab = cl(3, 4, 1);
    let brace = brace_cells(&[stair]);
    let surface = surface_with_slabs(&[overhead_slab]);
    let pos = Position::new(stair);

    let result = terrain_braces(pos, StanceKind::Standing, &brace, &surface);
    assert!(
        !*result,
        "Standing must NOT earn the terrain brace (C3 stance gate)",
    );
}

/// A [`StanceKind::Prone`] shooter on the same stair / slab geometry does NOT
/// earn the terrain brace (C3 — a Prone occupant is also single-cell on a stair,
/// so the dual-cell brace relationship is absent).
#[test]
fn prone_does_not_brace() {
    let stair = cl(3, 4, 0);
    let overhead_slab = cl(3, 4, 1);
    let brace = brace_cells(&[stair]);
    let surface = surface_with_slabs(&[overhead_slab]);
    let pos = Position::new(stair);

    let result = terrain_braces(pos, StanceKind::Prone, &brace, &surface);
    assert!(
        !*result,
        "Prone must NOT earn the terrain brace (C3 stance gate)",
    );
}

// ── C4 — revocation ─────────────────────────────────────────────────────

/// A kneeling stair occupant with a [`SlabState::Destroyed`] overhead slab loses
/// the terrain brace (C4: a smashed slab immediately revokes at the next fire).
#[test]
fn destroyed_overhead_slab_revokes_brace() {
    let stair = cl(3, 4, 0);
    let overhead_slab = cl(3, 4, 1);
    let brace = brace_cells(&[stair]);
    // Destroyed slab: set Present first, then destroy.
    let mut surface = SurfaceGrid::new();
    surface.set_slab(overhead_slab, SlabState::Present);
    surface.destroy_slab(overhead_slab);
    let pos = Position::new(stair);

    let result = terrain_braces(pos, StanceKind::Crouching, &brace, &surface);
    assert!(
        !*result,
        "a Destroyed overhead slab must revoke the terrain brace (C4)",
    );
}

// ── Non-stair / absent-overhead cases ───────────────────────────────────

/// A kneeling shooter on a cell NOT in the brace set (not a lower stair endpoint)
/// does NOT earn the terrain brace.
#[test]
fn non_stair_cell_no_brace() {
    let non_stair = cl(5, 6, 0);
    let overhead_slab = cl(5, 6, 1);
    // The brace set does NOT include non_stair — only a different cell.
    let brace = brace_cells(&[cl(1, 2, 0)]);
    let surface = surface_with_slabs(&[overhead_slab]);
    let pos = Position::new(non_stair);

    let result = terrain_braces(pos, StanceKind::Crouching, &brace, &surface);
    assert!(
        !*result,
        "a cell NOT in the brace-stair set must NOT earn the terrain brace",
    );
}

/// A kneeling lower-stair occupant with NO authored slab directly overhead
/// ([`SlabState::Absent`]) does NOT earn the terrain brace.
#[test]
fn absent_overhead_slab_no_brace() {
    let stair = cl(3, 4, 0);
    // No slab authored at (3, 4, 1) → Absent.
    let brace = brace_cells(&[stair]);
    let surface = SurfaceGrid::new();
    let pos = Position::new(stair);

    let result = terrain_braces(pos, StanceKind::Crouching, &brace, &surface);
    assert!(
        !*result,
        "an Absent overhead slab (no authored slab) must NOT earn the terrain brace",
    );
}

// ── Blocker-1 regression — upper stair endpoint does not brace ───────────

/// A multi-storey-span stair (`from.z = 0`, `to.z = 2`): only the LOWER endpoint
/// (storey 0) earns the terrain brace; the upper endpoint (storey 2) does NOT, even
/// with a `Present` slab at storey 3.
///
/// Proves the phantom-brace-at-wrong-slab fix (Blocker 1 from GTW-392 design).
#[test]
fn upper_stair_endpoint_does_not_brace() {
    let lower = cl(2, 3, 0); // lower endpoint: brace-eligible
    let upper = cl(2, 3, 2); // upper endpoint: NOT brace-eligible

    // The brace set contains ONLY the lower endpoint (design invariant).
    let brace = brace_cells(&[lower]);

    // Both a Present slab above the lower AND above the upper are authored.
    let overhead_lower = cl(2, 3, 1); // correct stair-brace slab
    let overhead_upper = cl(2, 3, 3); // phantom slab at upper storey+1
    let surface = surface_with_slabs(&[overhead_lower, overhead_upper]);

    // Lower endpoint + Crouching → braces.
    let result_lower = terrain_braces(
        Position::new(lower),
        StanceKind::Crouching,
        &brace,
        &surface,
    );
    assert!(
        *result_lower,
        "lower stair endpoint must earn the terrain brace (Blocker-1 regression)",
    );

    // Upper endpoint + Crouching → does NOT brace (NOT in brace set).
    let result_upper = terrain_braces(
        Position::new(upper),
        StanceKind::Crouching,
        &brace,
        &surface,
    );
    assert!(
        !*result_upper,
        "upper stair endpoint must NOT earn the terrain brace (Blocker-1 phantom-brace fix)",
    );
}

// ── `cell_above` edge cases ──────────────────────────────────────────────

/// `cell_above` returns `None` for a top-storey cell (z = `MAX_LEVELS - 1`).
#[test]
fn cell_above_top_storey_is_none() {
    let top = cl(0, 0, MAX_LEVELS - 1);
    assert!(
        cell_above(top).is_none(),
        "cell_above must return None for a top-storey cell (no overhead slab possible)",
    );
}

/// `cell_above` returns the correct `(x, y, z + 1)` cell for a non-top cell.
#[test]
fn cell_above_non_top_storey() {
    let cell = cl(3, 7, 2);
    let expected = cl(3, 7, 3);
    assert_eq!(
        cell_above(cell),
        Some(expected),
        "cell_above must return (x, y, z+1) for a non-top storey",
    );
}

// ── Parity — terrain-brace earns the same gate result as stable ──────────

/// [`TerrainBraced::new(true)`] has the same boolean truth value as [`Stable::new(true)`]
/// for parity — both drive the same `brace_engages` OR-clause.
///
/// This is a structural check: both carry `true` and both deref to `true`.
#[test]
fn terrain_braced_true_matches_stable_true_in_deref() {
    use crate::weapon::Stable;
    let terrain = TerrainBraced::new(true);
    let stable = Stable::new(true);
    assert_eq!(
        *terrain, *stable,
        "TerrainBraced(true) must deref to the same bool as Stable(true)"
    );

    let terrain_false = TerrainBraced::new(false);
    let stable_false = Stable::new(false);
    assert_eq!(
        *terrain_false, *stable_false,
        "TerrainBraced(false) must deref to the same bool as Stable(false)"
    );
}
