//! each terrain piece's own SPEC VARIANT, NOT from which authoring list (`walls` vs.
use super::support::*;
use crate::test_support::test_pieces;

fn crossed_terrain_fixture() -> (Situation, CellLevel, CellLevel) {
    let cover_in_walls_cell = key(1, 2, 0);
    let wall_in_scatter_cell = key(3, 4, 0);

    let mut situation = SituationBuilder::new()
        .with_gangers([ganger_at(key(10, 10, 0), 0), ganger_at(key(12, 12, 0), 1)])
        .build();

    situation
        .walls
        .push(CoverSpawn::new(cover_in_walls_cell, test_pieces::COVER));
    situation
        .scatter
        .push(CoverSpawn::new(wall_in_scatter_cell, test_pieces::WALL));

    (situation, cover_in_walls_cell, wall_in_scatter_cell)
}

#[test]
fn occupancy_kind_comes_from_spec_variant_not_authoring_list() {
    let (situation, cover_in_walls_cell, wall_in_scatter_cell) = crossed_terrain_fixture();
    let Some((mut app, _setup)) = run_setup(situation) else {
        return;
    };

    let world: &mut World = app.world_mut();
    let grid = world.get_resource::<OccupancyGrid>();
    assert!(
        grid.is_some(),
        "setup_battle must insert the OccupancyGrid resource",
    );
    let Some(grid) = grid else {
        return;
    };

    assert_eq!(
        grid.terrain(&cover_in_walls_cell),
        TerrainKind::Cover,
        "a Cover-spec piece authored in the walls list must read its DEF's kind \
         (Cover), not the Wall kind implied by the walls list",
    );
    assert_eq!(
        grid.terrain(&wall_in_scatter_cell),
        TerrainKind::Wall,
        "a Wall-spec piece authored in the scatter list must read its DEF's kind \
         (Wall), not the Cover kind implied by the scatter list",
    );
}
