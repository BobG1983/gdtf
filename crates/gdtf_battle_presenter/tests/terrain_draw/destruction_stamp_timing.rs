use std::time::Duration;

use bevy::{app::App, ecs::message::Messages, time::TimeUpdateStrategy};
use gdtf_battle_presenter::StampedGraphic;
use gdtf_battle_sim::{
    battle::BattleReady,
    cover::CoverLedger,
    entity::TerrainPieceKind,
    occupancy_sync::TerrainPieceDestroyed,
    prelude::{BattleInProgress, Cell, CellLevel, Level},
    surface::{SlabState, SurfaceGrid},
};

use super::harness::*;

fn draw_two_slabs(app: &mut App, slab_key: CellLevel, other_slab_key: CellLevel) {
    insert_occupancy(app, Vec::new());
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
}

fn absent_slab(app: &mut App, key: CellLevel) {
    if let Some(mut surface) = app.world_mut().get_resource_mut::<SurfaceGrid>() {
        surface.set_slab(key, SlabState::Absent);
    }
}

#[test]
fn the_stamp_waits_until_the_cursor_plays_the_smash() {
    let mut app = headless_renderer_app();
    settle_resources(&mut app);

    let l0 = Level::new(0);
    let slab_key = CellLevel::new(Cell::new(4, 5), l0);
    let other_slab_key = CellLevel::new(Cell::new(6, 7), l0);
    // A real piece at each cell: one for the smash to despawn, one the smash leaves alone.
    spawn_terrain_entity(&mut app, slab_key, "slab", None);
    spawn_terrain_entity(&mut app, other_slab_key, "slab", None);
    draw_two_slabs(&mut app, slab_key, other_slab_key);

    let defs = sprite_defs(&app);
    assert!(
        defs.is_some(),
        "the SpriteDefRegistry must be resident after settle"
    );
    let Some(defs) = defs else { return };

    assert_ne!(
        def_rect(&defs, "floor_alt_panel"),
        def_rect(&defs, "slab"),
        "the successor's def rect must differ from the intact slab rect, or the stamp is \
         invisible on this cell",
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

    // What the sim does at resolve time: the smash deed, the raw message, and the sim state.
    let smash = detained_smash_log(&mut app, &[(slab_key, TerrainPieceKind::Slab)]);
    app.world_mut()
        .resource_mut::<Messages<TerrainPieceDestroyed>>()
        .write(TerrainPieceDestroyed::new(slab_key, TerrainPieceKind::Slab));
    despawn_terrain_entity(&mut app, slab_key);
    absent_slab(&mut app, slab_key);
    spawn_terrain_entity(&mut app, slab_key, "floor_alt_panel", None);

    app.insert_resource(TimeUpdateStrategy::ManualDuration(Duration::ZERO));
    app.update();

    assert!(
        holding(&app),
        "the cursor must be holding on the detaining minor deed, or nothing keeps the smash \
         unplayed and this case measures no timing at all",
    );
    assert_eq!(
        shown(&app),
        smash,
        "the cursor must be waiting ON the smash entry, not past it",
    );
    assert_eq!(
        sprite_rect_at(&mut app, slab_key),
        def_rect(&defs, "slab"),
        "the cell must still draw the INTACT slab while the cursor holds short of the smash — \
         the raw message the sim wrote at resolve time must draw nothing on its own, and \
         neither must the successor already standing in the world — found {:?}",
        sprite_rect_at(&mut app, slab_key),
    );

    play_past(&mut app, smash, |_| {});

    assert_eq!(
        sprite_rect_at(&mut app, slab_key),
        def_rect(&defs, "floor_alt_panel"),
        "once the cursor plays the smash, the cell's sprite must carry the successor's \
         `floor_alt_panel` rect — found {:?}",
        sprite_rect_at(&mut app, slab_key),
    );
    assert_eq!(
        stamped_graphic_at(&mut app, slab_key),
        Some(StampedGraphic::from_key("floor_alt_panel")),
        "a played destruction leaves the cell stamped with the successor's graphic key — \
         found {:?}",
        stamped_graphic_at(&mut app, slab_key),
    );
    assert_eq!(
        sprite_entity_at(&mut app, slab_key),
        entity_before,
        "the smashed cell must be the SAME Entity after the stamp (no despawn/respawn)",
    );
    assert_eq!(
        sprite_rect_at(&mut app, other_slab_key),
        other_rect_before,
        "the other (intact) slab sprite must be untouched by the destruction",
    );
}

#[test]
fn the_stamp_runs_with_only_the_played_destroyed_buffer_in_the_app() {
    let mut app = headless_renderer_app_without_raw_destroyed();
    settle_resources(&mut app);

    let l0 = Level::new(0);
    let slab_key = CellLevel::new(Cell::new(4, 5), l0);
    let other_slab_key = CellLevel::new(Cell::new(6, 7), l0);
    spawn_terrain_entity(&mut app, slab_key, "slab", None);
    draw_two_slabs(&mut app, slab_key, other_slab_key);

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
    assert!(
        !raw_destroyed_present(&app),
        "this case's app must never register the raw TerrainPieceDestroyed buffer",
    );

    let smash = detained_smash_log(&mut app, &[(slab_key, TerrainPieceKind::Slab)]);
    absent_slab(&mut app, slab_key);
    despawn_terrain_entity(&mut app, slab_key);
    play_past(&mut app, smash, |app| {
        assert!(
            !raw_destroyed_present(app),
            "the raw TerrainPieceDestroyed buffer must stay absent across every update, or this \
             case cannot tell a gate on the raw buffer from a gate on the played one",
        );
    });

    assert_eq!(
        sprite_rect_at(&mut app, slab_key),
        None,
        "the log alone must drive the stamp: a gate left on the raw TerrainPieceDestroyed \
         buffer never runs stamp_destroyed_cell in this app, leaving the cell on the intact \
         slab rect — found {:?}",
        sprite_rect_at(&mut app, slab_key),
    );
    assert_eq!(
        stamped_graphic_at(&mut app, slab_key),
        Some(StampedGraphic::Marker),
        "the cell holding no slab, no piece and no theme default must be stamped the \
         missing-tile marker with no raw buffer in the app — found {:?}",
        stamped_graphic_at(&mut app, slab_key),
    );
}
