use bevy::ecs::message::Messages;
use gdtf_battle_sim::{
    battle::BattleReady,
    cover::CoverLedger,
    occupancy::{TerrainKind, TerrainPlacement},
    openable::OpenState,
    prelude::{BattleInProgress, Cell, CellLevel, Level},
    surface::SurfaceGrid,
    terrain::facing::TerrainFacing,
    test_support::test_pieces,
};
use gdtf_content_families::sprites::SpriteDefRegistry;

use super::harness::*;

// The `Open(North)` and `Open(East)` rows `test_door()` authors.
const DOOR_OPEN_NORTH: &str = "door_ns";
const DOOR_OPEN_EAST: &str = "door_ew";

#[test]
fn door_and_stair_orientation_tiles_resolve_to_distinct_sprites() {
    let mut app = headless_renderer_app();
    settle_resources(&mut app);

    let l0 = Level::new(0);
    let door_ns = CellLevel::new(Cell::new(3, 3), l0);
    let door_ew = CellLevel::new(Cell::new(4, 3), l0);

    insert_occupancy(
        &mut app,
        vec![
            TerrainPlacement::new(door_ns, TerrainKind::Wall),
            TerrainPlacement::new(door_ew, TerrainKind::Wall),
        ],
    );
    app.world_mut().insert_resource(CoverLedger::new());
    app.world_mut().insert_resource(SurfaceGrid::new());
    app.world_mut().insert_resource(BattleInProgress);

    for (at, facing) in [
        (door_ns, TerrainFacing::North),
        (door_ew, TerrainFacing::East),
    ] {
        let door = spawn_terrain_entity(&mut app, at, test_pieces::DOOR, facing, None);
        app.world_mut().entity_mut(door).insert(OpenState::Open);
    }

    app.world_mut()
        .resource_mut::<Messages<BattleReady>>()
        .write(BattleReady);
    app.update();

    let defs = sprite_defs(&app);
    assert!(
        defs.is_some(),
        "the SpriteDefRegistry must be resident after settle"
    );
    let Some(defs) = defs else { return };

    let open_north = def_rect(&defs, DOOR_OPEN_NORTH);
    let open_east = def_rect(&defs, DOOR_OPEN_EAST);
    assert!(
        open_north.is_some() && open_east.is_some(),
        "`{DOOR_OPEN_NORTH}` and `{DOOR_OPEN_EAST}` must both resolve to a sheet rect, or the \
         comparisons below pass on two missing-tile markers — got {open_north:?} and \
         {open_east:?}",
    );
    assert_ne!(
        open_north, open_east,
        "the two orientation tiles must draw different rects, or a cell turned east cannot be \
         told from one turned north",
    );

    assert_eq!(
        sprite_rect_at(&mut app, door_ns),
        open_north,
        "the north-facing open door must draw its def's `Open(North)` view, the `door_ns` \
         sheet rect",
    );
    assert_eq!(
        sprite_rect_at(&mut app, door_ew),
        open_east,
        "the east-facing open door must draw its def's `Open(East)` view, the `door_ew` sheet \
         rect — both doors are open, so a resolver that drops the facing stamps one view on \
         both cells",
    );

    assert_orientation_rects_all_distinct(&defs);
}

fn assert_orientation_rects_all_distinct(defs: &SpriteDefRegistry) {
    let rect = |name: &str| {
        let rect = def_rect(defs, name);
        assert!(
            rect.is_some(),
            "the `{name}` seeded def must carry a Sheet rect"
        );
        rect
    };
    let new_rects = [
        ("door_ns", rect("door_ns")),
        ("door_ew", rect("door_ew")),
        ("stair_ns_up", rect("stair_ns_up")),
        ("stair_ns_down", rect("stair_ns_down")),
        ("stair_ew_up", rect("stair_ew_up")),
        ("stair_ew_down", rect("stair_ew_down")),
    ];
    let existing = [
        ("wall", rect("wall")),
        ("wall_ew", rect("wall_ew")),
        ("cover", rect("cover")),
        ("slab", rect("slab")),
        ("floor", rect("floor")),
        ("rubble", rect("rubble")),
        ("door", rect("door")),
        ("stair_up", rect("stair_up")),
        ("stair_down", rect("stair_down")),
        ("ladder", rect("ladder")),
    ];
    for (i, (na, a)) in new_rects.iter().enumerate() {
        for (nb, b) in new_rects.iter().skip(i + 1) {
            assert_ne!(
                a, b,
                "the new orientation tiles {na} and {nb} must be DISTINCT sprites (collapsing \
                 two onto one rect would mean an orientation/direction is indistinguishable)",
            );
        }
        for (ne, e) in &existing {
            assert_ne!(
                a, e,
                "the new orientation tile {na} must be DISTINCT from the existing {ne} tile \
                 (it is its own AUTHORED sprite, never a reuse of an existing terrain rect)",
            );
        }
    }
}
