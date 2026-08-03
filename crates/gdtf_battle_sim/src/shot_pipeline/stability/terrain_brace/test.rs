use bevy::platform::collections::HashSet;

use super::{gate::cell_above, *};
use crate::{
    ganger::{Position, StanceKind},
    metric::{Cell, CellLevel, Level, MAX_LEVELS},
    slab::BraceStairCells,
    surface::{SlabState, SurfaceGrid},
};

fn cl(x: i32, y: i32, z: u8) -> CellLevel {
    CellLevel::new(Cell::new(x, y), Level::new(z))
}

fn brace_cells(cells: &[CellLevel]) -> BraceStairCells {
    BraceStairCells::new(cells.iter().copied().collect::<HashSet<_>>())
}

fn surface_with_slabs(present: &[CellLevel]) -> SurfaceGrid {
    let mut grid = SurfaceGrid::new();
    for &cell in present {
        grid.set_slab(cell, SlabState::Present);
    }
    grid
}


#[test]
fn kneeling_on_brace_stair_under_present_slab_braces() {
    let stair = cl(3, 4, 0);
    let overhead_slab = cl(3, 4, 1); 
    let brace = brace_cells(&[stair]);
    let surface = surface_with_slabs(&[overhead_slab]);
    let pos = Position::new(stair);

    let result = terrain_braces(pos, StanceKind::Crouching, &brace, &surface);
    assert!(
        *result,
        "kneeling on a brace stair under a Present slab must earn the terrain brace",
    );
}


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


#[test]
fn destroyed_overhead_slab_revokes_brace() {
    let stair = cl(3, 4, 0);
    let overhead_slab = cl(3, 4, 1);
    let brace = brace_cells(&[stair]);
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


#[test]
fn non_stair_cell_no_brace() {
    let non_stair = cl(5, 6, 0);
    let overhead_slab = cl(5, 6, 1);
    let brace = brace_cells(&[cl(1, 2, 0)]);
    let surface = surface_with_slabs(&[overhead_slab]);
    let pos = Position::new(non_stair);

    let result = terrain_braces(pos, StanceKind::Crouching, &brace, &surface);
    assert!(
        !*result,
        "a cell NOT in the brace-stair set must NOT earn the terrain brace",
    );
}

#[test]
fn absent_overhead_slab_no_brace() {
    let stair = cl(3, 4, 0);
    let brace = brace_cells(&[stair]);
    let surface = SurfaceGrid::new();
    let pos = Position::new(stair);

    let result = terrain_braces(pos, StanceKind::Crouching, &brace, &surface);
    assert!(
        !*result,
        "an Absent overhead slab (no authored slab) must NOT earn the terrain brace",
    );
}


#[test]
fn upper_stair_endpoint_does_not_brace() {
    let lower = cl(2, 3, 0); 
    let upper = cl(2, 3, 2); 

    let brace = brace_cells(&[lower]);

    let overhead_lower = cl(2, 3, 1); 
    let overhead_upper = cl(2, 3, 3); 
    let surface = surface_with_slabs(&[overhead_lower, overhead_upper]);

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


#[test]
fn cell_above_top_storey_is_none() {
    let top = cl(0, 0, MAX_LEVELS - 1);
    assert!(
        cell_above(top).is_none(),
        "cell_above must return None for a top-storey cell (no overhead slab possible)",
    );
}

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
