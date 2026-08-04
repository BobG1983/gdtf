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

#[test]
fn destroyed_slab_adjacent_to_drawn_floor_emits_drop_depth() {
    let hole = key(5, 5, 2);
    let floor = key(6, 5, 2);

    let mut surface = SurfaceGrid::new();
    surface.set_slab(floor, SlabState::Present);
    surface.destroy_slab(hole);

    let occupancy = OccupancyGrid::new();

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

#[test]
fn ground_level_never_emits_drop_depth() {
    let hole = key(5, 5, 0);
    let surface = SurfaceGrid::new();
    let occupancy = OccupancyGrid::new();
    let drawn: HashSet<Cell> = std::iter::once(hole.cell()).collect();
    let squad = SquadVisibility::omniscient(&occupancy);

    let drops = gather_drop_depth(Level::new(0), &surface, &occupancy, &drawn, &squad);

    assert!(
        drops.is_empty(),
        "the ground storey can never be a drop-depth hole",
    );
}

#[test]
fn unexplored_hole_emits_no_drop_depth() {
    let hole = key(5, 5, 2);
    let floor = key(6, 5, 2);

    let mut surface = SurfaceGrid::new();
    surface.set_slab(floor, SlabState::Present);
    surface.destroy_slab(hole);

    let occupancy = OccupancyGrid::new();
    let drawn: HashSet<Cell> = std::iter::once(floor.cell()).collect();
    let squad = SquadVisibility::default();

    let drops = gather_drop_depth(Level::new(2), &surface, &occupancy, &drawn, &squad);

    assert!(
        drops.is_empty(),
        "an unexplored hole never surfaces a drop-depth badge",
    );
}
