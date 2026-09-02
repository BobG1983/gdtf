use bevy::ecs::message::Messages;
use gdtf_battle_presenter::{ActiveLevel, StampedGraphic};
use gdtf_battle_sim::{
    battle::BattleReady,
    cover::CoverLedger,
    entity::TerrainPieceKind,
    occupancy::{TerrainKind, TerrainPlacement},
    prelude::{BattleInProgress, Cell, CellLevel, Level},
    surface::SurfaceGrid,
};

use super::harness::*;

#[test]
fn a_played_smash_stamps_the_cover_cell_with_its_successors_key() {
    let mut app = headless_renderer_app();
    settle_resources(&mut app);

    let l0 = Level::new(0);
    let wall_key = CellLevel::new(Cell::new(8, 7), l0);
    let cover_key = CellLevel::new(Cell::new(9, 8), l0);

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
    assert_eq!(
        sprite_rect_at(&mut app, cover_key),
        def_rect(&defs, "cover"),
        "the cover cell must be drawn as cover before the smash",
    );
    let wall_rect_before = sprite_rect_at(&mut app, wall_key);

    // The successor the sim spawns for the smashed piece, standing at the same cell.
    spawn_terrain_entity(&mut app, cover_key, "floor_alt_panel", None);
    let smash = detained_smash_log(&mut app, &[(cover_key, TerrainPieceKind::Cover)]);
    play_past(&mut app, smash, |_| {});

    assert_eq!(
        sprite_rect_at(&mut app, cover_key),
        def_rect(&defs, "floor_alt_panel"),
        "the smashed cover cell must now carry the SUCCESSOR's `floor_alt_panel` rect, not a \
         role key written into the reader — found {:?}",
        sprite_rect_at(&mut app, cover_key),
    );
    assert_eq!(
        stamped_graphic_at(&mut app, cover_key),
        Some(StampedGraphic::from_key("floor_alt_panel")),
        "the stamp names what graphic_name_at answered for the cell, which is the successor \
         piece's own graphic key — found {:?}",
        stamped_graphic_at(&mut app, cover_key),
    );
    assert_eq!(
        sprite_rect_at(&mut app, wall_key),
        wall_rect_before,
        "the other (wall) terrain sprite must be untouched by the cover destruction",
    );
}

#[test]
fn a_played_smash_stamps_a_drawn_lower_storey_and_ignores_above_active() {
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

    spawn_terrain_entity(&mut app, lower_cover, "floor_alt_panel", None);
    let smash = detained_smash_log(
        &mut app,
        &[
            (lower_cover, TerrainPieceKind::Cover),
            (above_cover, TerrainPieceKind::Cover),
        ],
    );
    play_past(&mut app, smash, |_| {});

    assert_eq!(
        sprite_rect_at(&mut app, lower_cover),
        def_rect(&defs, "floor_alt_panel"),
        "a cover smashed on a DRAWN lower storey must be stamped with its successor's key — \
         found {:?}",
        sprite_rect_at(&mut app, lower_cover),
    );
    assert_eq!(
        stamped_graphic_at(&mut app, lower_cover),
        Some(StampedGraphic::from_key("floor_alt_panel")),
        "the lower-storey cell must be stamped with the successor's graphic key — found {:?}",
        stamped_graphic_at(&mut app, lower_cover),
    );
    assert_eq!(
        sprite_rect_at(&mut app, above_cover),
        None,
        "a cover smashed STRICTLY ABOVE the active view level stays undrawn — the stamp \
         retargets an existing tile and never spawns one",
    );
}

#[test]
fn a_played_stamp_and_a_forced_redraw_agree_on_a_pieceless_level_0_cell() {
    let mut app = headless_renderer_app();
    settle_resources(&mut app);

    let pieceless = CellLevel::new(Cell::new(4, 4), Level::new(0));

    // No TerrainPlacement anywhere, so role_at answers Floor once the piece is gone.
    spawn_terrain_entity(&mut app, pieceless, "wall", None);
    insert_occupancy(&mut app, Vec::new());
    app.world_mut().insert_resource(CoverLedger::new());
    app.world_mut().insert_resource(SurfaceGrid::new());
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
        def_rect(&defs, "wall"),
        def_rect(&defs, "floor"),
        "the wall and floor def rects must differ, or a stamp that never runs would look like \
         one that agrees with the redraw",
    );
    assert_eq!(
        sprite_rect_at(&mut app, pieceless),
        def_rect(&defs, "wall"),
        "precondition: the cell draws its piece's `wall` key before the smash",
    );

    despawn_terrain_entity(&mut app, pieceless);
    let smash = detained_smash_log(&mut app, &[(pieceless, TerrainPieceKind::Wall)]);
    play_past(&mut app, smash, |_| {});
    let stamped_rect = sprite_rect_at(&mut app, pieceless);

    app.world_mut()
        .resource_mut::<Messages<BattleReady>>()
        .write(BattleReady);
    app.update();

    assert_eq!(
        sprite_rect_at(&mut app, pieceless),
        stamped_rect,
        "on a level-0 cell left with no piece, the played stamp and a full redraw must answer \
         the same graphic — the stamp resolves through graphic_name_at, the same function \
         draw_static_battlefield calls — found {:?} after the redraw",
        sprite_rect_at(&mut app, pieceless),
    );
}
