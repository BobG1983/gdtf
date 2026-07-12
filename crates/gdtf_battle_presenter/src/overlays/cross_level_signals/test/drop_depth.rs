//! GTW-596 `DropDepth` gathering: the per-producer positive assertion (acceptance
//! clause 2) — a destroyed-slab hole adjacent to drawn floor emits a `DropDepth`
//! badge with the REAL `resolve_drop` fall distance, plus the ground-floor guard
//! and the fog-gating negative case.

use bevy::platform::collections::HashSet;
use gdtf_battle_sim::{
    falls::StoreysFallen,
    prelude::{Cell, CellLevel, Level, OccupancyGrid},
    surface::{SlabState, SurfaceGrid},
    visibility::SquadVisibility,
};

use crate::overlays::cross_level_signals::drop_depth::gather_drop_depth;

fn key(x: i32, y: i32, z: u8) -> CellLevel {
    CellLevel::new(Cell::new(x, y), Level::new(z))
}

/// Acceptance clause 2 (`DropDepth`): a destroyed slab bordered by an intact,
/// drawn floor cell, on a fog-EXPLORED footprint, emits a `DropDepth` badge with
/// the real `resolve_drop` landing distance.
#[test]
fn destroyed_slab_adjacent_to_drawn_floor_emits_drop_depth() {
    let hole = key(5, 5, 2);
    let floor = key(6, 5, 2); // drawn neighbour — floor slab present

    let mut surface = SurfaceGrid::new();
    surface.set_slab(floor, SlabState::Present);
    surface.destroy_slab(hole); // the hole: a slab that existed and was smashed

    let occupancy = OccupancyGrid::new(); // every untouched cell reads TerrainKind::Open

    let drawn: HashSet<Cell> = std::iter::once(floor.cell()).collect();
    let explored: HashSet<CellLevel> = [hole, floor].into_iter().collect();
    let squad = SquadVisibility::new(explored.clone(), explored);

    let drops = gather_drop_depth(Level::new(2), &surface, &occupancy, &drawn, &squad);

    assert_eq!(
        drops,
        vec![(hole.cell(), StoreysFallen::new(2))],
        "a destroyed slab bordering drawn floor emits DropDepth at its real \
         resolve_drop landing distance (falls all the way to the ground, 2 storeys)",
    );
}

/// Ground level (0) never drops — `resolve_drop`'s own guard — so the gather
/// short-circuits to empty even over an otherwise-qualifying grid.
#[test]
fn ground_level_never_emits_drop_depth() {
    let hole = key(5, 5, 0);
    let surface = SurfaceGrid::new(); // Absent by default — would "qualify" at level >= 1
    let occupancy = OccupancyGrid::new();
    let drawn: HashSet<Cell> = std::iter::once(hole.cell()).collect();
    let squad = SquadVisibility::omniscient(&occupancy);

    let drops = gather_drop_depth(Level::new(0), &surface, &occupancy, &drawn, &squad);

    assert!(
        drops.is_empty(),
        "the ground storey can never be a drop-depth hole",
    );
}

/// A hole cell the squad has NEVER seen (not even EXPLORED) does not surface a
/// `DropDepth` badge — terrain facts stay gated like the terrain draw itself
/// (acceptance clause 1's fog-gating principle, applied to terrain facts).
#[test]
fn unexplored_hole_emits_no_drop_depth() {
    let hole = key(5, 5, 2);
    let floor = key(6, 5, 2);

    let mut surface = SurfaceGrid::new();
    surface.set_slab(floor, SlabState::Present);
    surface.destroy_slab(hole);

    let occupancy = OccupancyGrid::new();
    let drawn: HashSet<Cell> = std::iter::once(floor.cell()).collect();
    let squad = SquadVisibility::default(); // nothing explored

    let drops = gather_drop_depth(Level::new(2), &surface, &occupancy, &drawn, &squad);

    assert!(
        drops.is_empty(),
        "an unexplored hole never surfaces a drop-depth badge",
    );
}
