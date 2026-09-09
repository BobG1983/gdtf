use super::support::*;
use crate::{
    occupancy::OccupancyGrid,
    terrain::entity::{BlocksPathfinding, TerrainCell},
    test_support::{SituationBuilder, ganger_at, setup_request},
};

#[test]
fn wall_is_path_blocked_after_setup() {
    let wall_cell = cl(2, 2, 0);
    let slab_cell = cl(3, 4, 1);
    let open_cell = cl(5, 5, 0);

    let mut app = headless_app();
    let situation = SituationBuilder::new()
        .with_ganger(ganger_at(cl(0, 0, 0), 0))
        .with_ganger(ganger_at(cl(1, 1, 0), 1))
        .wall_at(wall_cell)
        .slab_at(slab_cell)
        .build();
    app.world_mut()
        .write_message(setup_request(situation, crate::rng::BattleSeed::new(0x07)));
    app.update();

    let world = app.world_mut();

    let mut marker_query = world.query::<(&TerrainCell, Option<&BlocksPathfinding>)>();
    let wall_has_marker = marker_query
        .iter(world)
        .find(|(cell, _)| ***cell == wall_cell)
        .map(|(_, m)| m.is_some());
    assert_eq!(
        wall_has_marker,
        Some(true),
        "C1: the spawned wall entity must carry the derived BlocksPathfinding marker",
    );

    let grid = world.get_resource::<OccupancyGrid>();
    assert!(grid.is_some(), "setup must insert the OccupancyGrid");
    let Some(grid) = grid else {
        return;
    };
    assert!(
        *grid.is_path_blocked(&wall_cell),
        "C3/C5: the wall cell must be PATH-blocked after the projection runs",
    );
    assert!(
        *grid.is_blocked(&wall_cell),
        "C5: the wall cell is ALSO kind-blocked (vision) — path & vision agree for a wall",
    );
    assert!(
        !*grid.is_path_blocked(&slab_cell),
        "C1: a slab does NOT block the path by default (no marker → not in the surface)",
    );
    assert!(
        !*grid.is_path_blocked(&open_cell),
        "an empty cell is not path-blocked",
    );
}
