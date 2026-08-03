use bevy::ecs::message::Messages;
use gdtf_battle_sim::{
    battle::BattleReady,
    cover::CoverLedger,
    occupancy::{TerrainKind, TerrainPlacement},
    prelude::{BattleInProgress, Cell, CellLevel, Level},
    surface::{SlabState, SurfaceGrid},
};
use gdtf_content_families::sprites::SpriteDefRegistry;

use super::harness::*;

#[test]
fn door_and_stair_orientation_tiles_resolve_to_distinct_sprites() {
    let mut app = headless_renderer_app();
    settle_resources(&mut app);

    let l0 = Level::new(0);
    let door_ns = CellLevel::new(Cell::new(3, 3), l0);
    let door_ew = CellLevel::new(Cell::new(4, 3), l0);
    let stair_ns_up = CellLevel::new(Cell::new(5, 3), l0);
    let stair_ns_down = CellLevel::new(Cell::new(6, 3), l0);
    let stair_ew_up = CellLevel::new(Cell::new(7, 3), l0);
    let stair_ew_down = CellLevel::new(Cell::new(8, 3), l0);

    insert_occupancy(
        &mut app,
        vec![
            TerrainPlacement::new(door_ns, TerrainKind::Wall),
            TerrainPlacement::new(door_ew, TerrainKind::Wall),
        ],
    );
    app.world_mut().insert_resource(CoverLedger::new());
    let mut surface = SurfaceGrid::new();
    surface.set_slab(stair_ns_up, SlabState::Present);
    surface.set_slab(stair_ns_down, SlabState::Present);
    surface.set_slab(stair_ew_up, SlabState::Present);
    surface.set_slab(stair_ew_down, SlabState::Present);
    app.world_mut().insert_resource(surface);
    app.world_mut().insert_resource(BattleInProgress);

    spawn_terrain_entity(&mut app, door_ns, "door_ns", None);
    spawn_terrain_entity(&mut app, door_ew, "door_ew", None);
    spawn_terrain_entity(&mut app, stair_ns_up, "stair_ns_up", None);
    spawn_terrain_entity(&mut app, stair_ns_down, "stair_ns_down", None);
    spawn_terrain_entity(&mut app, stair_ew_up, "stair_ew_up", None);
    spawn_terrain_entity(&mut app, stair_ew_down, "stair_ew_down", None);

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

    assert_eq!(
        sprite_rect_at(&mut app, door_ns),
        def_rect(&defs, "door_ns"),
        "the NS-door cell must draw the `door_ns` def's sheet rect",
    );
    assert_eq!(
        sprite_rect_at(&mut app, door_ew),
        def_rect(&defs, "door_ew"),
        "the EW-door cell must draw the `door_ew` def's sheet rect",
    );
    assert_eq!(
        sprite_rect_at(&mut app, stair_ns_up),
        def_rect(&defs, "stair_ns_up"),
        "the NS-up-stair cell must draw the `stair_ns_up` def's sheet rect",
    );
    assert_eq!(
        sprite_rect_at(&mut app, stair_ns_down),
        def_rect(&defs, "stair_ns_down"),
        "the NS-down-stair cell must draw the `stair_ns_down` def's sheet rect",
    );
    assert_eq!(
        sprite_rect_at(&mut app, stair_ew_up),
        def_rect(&defs, "stair_ew_up"),
        "the EW-up-stair cell must draw the `stair_ew_up` def's sheet rect",
    );
    assert_eq!(
        sprite_rect_at(&mut app, stair_ew_down),
        def_rect(&defs, "stair_ew_down"),
        "the EW-down-stair cell must draw the `stair_ew_down` def's sheet rect",
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
