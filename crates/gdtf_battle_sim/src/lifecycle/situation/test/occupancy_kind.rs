//! GTW-483 — the spawned [`OccupancyGrid`]'s per-slot [`TerrainKind`] is derived from
//! each terrain piece's own SPEC VARIANT, NOT from which authoring list (`walls` vs.
//! `scatter`) the piece happened to sit in.
//!
//! The bug these tests pin: `setup_battle` formerly re-derived the occupancy kind from
//! list membership (`walls` → `Wall`, `scatter` → `Cover`), so a `Cover`/`Scatter` spec
//! placed in the `walls` list wrongly read as `Wall` (corrupting move-cost +
//! destructibility), and a `Wall` spec in `scatter` wrongly read as `Cover`. The fix
//! carries the resolved `piece_kind` (spec-variant-derived) through into each
//! `TerrainPlacement`.

use super::support::*;
use crate::test_support::test_pieces;

/// Author a `Cover` spec (`test-cover`) in the `walls` list and a `Wall` spec
/// (`test-wall`) in the `scatter` list — the cross-placement fixture. Returns the
/// situation plus the two distinct cells so each can be read back independently.
fn crossed_terrain_fixture() -> (Situation, CellLevel, CellLevel) {
    // The cover-spec piece, deliberately authored in the WALLS list.
    let cover_in_walls_cell = key(1, 2, 0);
    // The wall-spec piece, deliberately authored in the SCATTER list.
    let wall_in_scatter_cell = key(3, 4, 0);

    // Two gangers away from the terrain cells so neither slot is occupant-keyed.
    let mut situation = SituationBuilder::new()
        .with_gangers([ganger_at(key(10, 10, 0), 0), ganger_at(key(12, 12, 0), 1)])
        .build();

    // A Cover def UUID in the `walls` list — list membership says "Wall",
    // the def's own sim-kind variant says "Cover".
    situation
        .walls
        .push(CoverSpawn::new(cover_in_walls_cell, test_pieces::COVER));
    // A Wall def UUID in the `scatter` list — list membership says "Cover", the def's
    // own sim-kind variant says "Wall".
    situation
        .scatter
        .push(CoverSpawn::new(wall_in_scatter_cell, test_pieces::WALL));

    (situation, cover_in_walls_cell, wall_in_scatter_cell)
}

/// C1/C2 — the occupancy slot of a `Cover` spec authored in the `walls` list reads
/// [`TerrainKind::Cover`] (its DEF's kind), and a `Wall` spec authored in the `scatter`
/// list reads [`TerrainKind::Wall`] (its DEF's kind) — NOT the kind implied by which
/// list the piece sat in.
///
/// Pin-discriminating (C2): under the reverted list-membership re-derive the `walls`
/// piece reads `Wall` and the `scatter` piece reads `Cover`, so BOTH assertions fail.
/// This drives the REAL [`setup_battle`] path end-to-end (no reimplementation).
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

    // The Cover spec sitting in the WALLS list reads Cover (its def's kind), not Wall.
    assert_eq!(
        grid.terrain(&cover_in_walls_cell),
        TerrainKind::Cover,
        "a Cover-spec piece authored in the walls list must read its DEF's kind \
         (Cover), not the Wall kind implied by the walls list",
    );
    // The Wall spec sitting in the SCATTER list reads Wall (its def's kind), not Cover.
    assert_eq!(
        grid.terrain(&wall_in_scatter_cell),
        TerrainKind::Wall,
        "a Wall-spec piece authored in the scatter list must read its DEF's kind \
         (Wall), not the Cover kind implied by the scatter list",
    );
}
