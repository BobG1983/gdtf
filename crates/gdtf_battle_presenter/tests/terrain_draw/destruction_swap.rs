use bevy::ecs::message::Messages;
use gdtf_battle_presenter::{ActiveLevel, Played, StampedGraphic};
use gdtf_battle_sim::{
    battle::BattleReady,
    cover::CoverLedger,
    entity::TerrainPieceKind,
    occupancy::{TerrainKind, TerrainPlacement},
    occupancy_sync::TerrainPieceDestroyed,
    prelude::{BattleInProgress, Cell, CellLevel, Level},
    surface::{SlabState, SurfaceGrid},
};

use super::harness::*;

#[test]
fn cover_destroyed_swaps_the_cover_cell_to_rubble() {
    let mut app = headless_renderer_app();
    settle_resources(&mut app);

    let wall_cell = Cell::new(8, 7);
    let cover_cell = Cell::new(9, 8);
    let l0 = Level::new(0);
    let wall_key = CellLevel::new(wall_cell, l0);
    let cover_key = CellLevel::new(cover_cell, l0);

    insert_occupancy(
        &mut app,
        vec![
            TerrainPlacement::new(wall_key, TerrainKind::Wall),
            TerrainPlacement::new(cover_key, TerrainKind::Cover),
        ],
    );
    let mut cover_ledger = CoverLedger::new();
    cover_ledger.insert(cover_key, low_cover_entry());
    app.world_mut().insert_resource(cover_ledger);
    app.world_mut().insert_resource(SurfaceGrid::new());
    app.world_mut().insert_resource(BattleInProgress);

    app.world_mut()
        .resource_mut::<Messages<BattleReady>>()
        .write(BattleReady);
    app.update();

    let defs = sprite_defs(&app);
    assert!(defs.is_some(), "the SpriteDefRegistry must be resident");
    let Some(defs) = defs else { return };
    let wall_rect_before = sprite_rect_at(&mut app, wall_key);

    app.world_mut()
        .resource_mut::<Messages<TerrainPieceDestroyed>>()
        .write(TerrainPieceDestroyed::new(
            cover_key,
            TerrainPieceKind::Cover,
        ));
    app.update();

    assert_eq!(
        sprite_rect_at(&mut app, cover_key),
        def_rect(&defs, "rubble"),
        "the destroyed cover cell's sprite must now carry the `rubble` def's rect",
    );
    assert_eq!(
        stamped_graphic_at(&mut app, cover_key),
        Some(StampedGraphic::from_key("rubble")),
        "a Cover-kind destruction leaves the cell stamped with the rubble role — found {:?}",
        stamped_graphic_at(&mut app, cover_key),
    );
    assert_eq!(
        sprite_rect_at(&mut app, wall_key),
        wall_rect_before,
        "the other (wall) terrain sprite must be untouched by the cover destruction",
    );
}

#[test]
fn slab_destroyed_swaps_the_slab_cell_to_destroyed_in_place() {
    let mut app = headless_renderer_app();
    settle_resources(&mut app);

    let slab_cell = Cell::new(4, 5);
    let other_slab_cell = Cell::new(6, 7);
    let l0 = Level::new(0);
    let slab_key = CellLevel::new(slab_cell, l0);
    let other_slab_key = CellLevel::new(other_slab_cell, l0);

    insert_occupancy(&mut app, Vec::new());
    app.world_mut().insert_resource(CoverLedger::new());
    let mut surface = SurfaceGrid::new();
    surface.set_slab(slab_key, SlabState::Present);
    surface.set_slab(other_slab_key, SlabState::Present);
    app.world_mut().insert_resource(surface);
    app.world_mut().insert_resource(BattleInProgress);

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

    assert_ne!(
        def_rect(&defs, "slab_destroyed"),
        def_rect(&defs, "slab"),
        "the slab_destroyed def rect must differ from the intact slab rect (a real swap)",
    );

    assert_eq!(
        sprite_rect_at(&mut app, slab_key),
        def_rect(&defs, "slab"),
        "the slab cell's sprite must start on the intact slab def's rect",
    );
    let entity_before = sprite_entity_at(&mut app, slab_key);
    assert!(
        entity_before.is_some(),
        "the slab cell's terrain sprite must exist before the destruction",
    );
    let other_rect_before = sprite_rect_at(&mut app, other_slab_key);

    app.world_mut()
        .resource_mut::<Messages<TerrainPieceDestroyed>>()
        .write(TerrainPieceDestroyed::new(slab_key, TerrainPieceKind::Slab));
    app.update();

    assert_eq!(
        sprite_rect_at(&mut app, slab_key),
        def_rect(&defs, "slab_destroyed"),
        "the destroyed slab cell's sprite must now carry the `slab_destroyed` def's rect",
    );
    assert_eq!(
        stamped_graphic_at(&mut app, slab_key),
        Some(StampedGraphic::from_key("slab_destroyed")),
        "a Slab-kind destruction leaves the cell stamped with the destroyed-slab role, not \
         rubble: the cover swap runs after the slab swap and must skip a Slab-kind message — \
         found {:?}",
        stamped_graphic_at(&mut app, slab_key),
    );
    assert_eq!(
        sprite_entity_at(&mut app, slab_key),
        entity_before,
        "the destroyed slab cell must be the SAME Entity after the swap (no despawn/respawn)",
    );
    assert_eq!(
        sprite_rect_at(&mut app, other_slab_key),
        other_rect_before,
        "the other (intact) slab sprite must be untouched by the slab destruction",
    );
}

#[test]
fn the_slab_swap_runs_with_only_the_raw_destroyed_buffer_in_the_app() {
    let mut app = headless_renderer_app();
    settle_resources(&mut app);

    let slab_key = CellLevel::new(Cell::new(4, 5), Level::new(0));

    insert_occupancy(&mut app, Vec::new());
    app.world_mut().insert_resource(CoverLedger::new());
    let mut surface = SurfaceGrid::new();
    surface.set_slab(slab_key, SlabState::Present);
    app.world_mut().insert_resource(surface);
    app.world_mut().insert_resource(BattleInProgress);

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
        sprite_rect_at(&mut app, slab_key),
        def_rect(&defs, "slab"),
        "the slab cell's sprite must start on the intact slab def's rect",
    );

    // TopDownRendererPlugin registers the played buffer, so only dropping it tells the gates apart.
    app.world_mut()
        .remove_resource::<Messages<Played<TerrainPieceDestroyed>>>();
    app.world_mut()
        .resource_mut::<Messages<TerrainPieceDestroyed>>()
        .write(TerrainPieceDestroyed::new(slab_key, TerrainPieceKind::Slab));
    app.update();

    assert!(
        !app.world()
            .contains_resource::<Messages<Played<TerrainPieceDestroyed>>>(),
        "the played buffer must stay absent across the update, or this case cannot tell a gate \
         on the raw buffer from a gate on the played one",
    );
    assert_eq!(
        stamped_graphic_at(&mut app, slab_key),
        Some(StampedGraphic::from_key("slab_destroyed")),
        "swap_destroyed_slab must be gated on the raw TerrainPieceDestroyed buffer it reads: \
         moving that gate onto Played<TerrainPieceDestroyed> before the reader moves disables \
         the system in an app that holds only the raw buffer — found {:?}",
        stamped_graphic_at(&mut app, slab_key),
    );
    assert_eq!(
        sprite_rect_at(&mut app, slab_key),
        def_rect(&defs, "slab_destroyed"),
        "the destroyed slab cell's sprite must carry the `slab_destroyed` def's rect with the \
         played buffer absent",
    );
}

#[test]
fn cover_destroyed_swaps_on_lower_storey_and_ignores_above_active() {
    let mut app = headless_renderer_app();
    settle_resources(&mut app);

    let l0 = Level::new(0);
    let l1 = Level::new(1);
    let l2 = Level::new(2);
    let lower_cover = CellLevel::new(Cell::new(9, 8), l0);
    let above_cover = CellLevel::new(Cell::new(9, 8), l2);

    insert_occupancy(
        &mut app,
        vec![
            TerrainPlacement::new(lower_cover, TerrainKind::Cover),
            TerrainPlacement::new(above_cover, TerrainKind::Cover),
        ],
    );
    let mut cover_ledger = CoverLedger::new();
    cover_ledger.insert(lower_cover, low_cover_entry());
    cover_ledger.insert(above_cover, low_cover_entry());
    app.world_mut().insert_resource(cover_ledger);
    app.world_mut().insert_resource(SurfaceGrid::new());
    app.world_mut().insert_resource(BattleInProgress);

    *app.world_mut().resource_mut::<ActiveLevel>() = ActiveLevel::new(l1);
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
        sprite_rect_at(&mut app, lower_cover),
        def_rect(&defs, "cover"),
        "the lower-storey cover must be drawn before the smash",
    );
    assert_eq!(
        sprite_rect_at(&mut app, above_cover),
        None,
        "the above-active cover must NOT be drawn (culled)",
    );

    app.world_mut()
        .resource_mut::<Messages<TerrainPieceDestroyed>>()
        .write(TerrainPieceDestroyed::new(
            lower_cover,
            TerrainPieceKind::Cover,
        ));
    app.world_mut()
        .resource_mut::<Messages<TerrainPieceDestroyed>>()
        .write(TerrainPieceDestroyed::new(
            above_cover,
            TerrainPieceKind::Cover,
        ));
    app.update();

    assert_eq!(
        sprite_rect_at(&mut app, lower_cover),
        def_rect(&defs, "rubble"),
        "a cover smashed on a DRAWN lower storey must swap to the rubble tile (C6)",
    );
    assert_eq!(
        sprite_rect_at(&mut app, above_cover),
        None,
        "a cover smashed STRICTLY ABOVE the active view level must be ignored (not drawn)",
    );
}
