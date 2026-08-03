use bevy::{app::App, ecs::message::Messages};
use gdtf_battle_sim::{
    battle::BattleReady,
    cover::CoverLedger,
    emplacement::EmplacementState,
    entity::TerrainCell,
    occupancy::{TerrainKind, TerrainPlacement},
    piece::TerrainGraphicKey,
    prelude::{BattleInProgress, Cell, CellLevel, Level},
    surface::SurfaceGrid,
};

use super::harness::*;

fn spawn_emplacement_entity(app: &mut App, key: CellLevel) -> bevy::ecs::entity::Entity {
    app.world_mut()
        .spawn((
            TerrainCell::new(key),
            TerrainGraphicKey::new("emplacement".to_owned()),
            EmplacementState::Vacant,
        ))
        .id()
}

#[test]
fn emplacement_draws_its_own_distinct_tile() {
    let mut app = headless_renderer_app();
    settle_resources(&mut app);

    let l0 = Level::new(0);
    let emp_cell = CellLevel::new(Cell::new(8, 7), l0);
    let cover_cell = CellLevel::new(Cell::new(9, 8), l0);

    insert_occupancy(
        &mut app,
        vec![
            TerrainPlacement::new(emp_cell, TerrainKind::Emplacement),
            TerrainPlacement::new(cover_cell, TerrainKind::Cover),
        ],
    );
    let mut cover_ledger = CoverLedger::new();
    cover_ledger.insert(emp_cell, low_cover_entry());
    cover_ledger.insert(cover_cell, low_cover_entry());
    app.world_mut().insert_resource(cover_ledger);
    app.world_mut().insert_resource(SurfaceGrid::new());
    app.world_mut().insert_resource(BattleInProgress);

    spawn_emplacement_entity(&mut app, emp_cell);
    spawn_terrain_entity(&mut app, cover_cell, "cover", None);

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
        def_rect(&defs, "emplacement"),
        def_rect(&defs, "cover"),
        "the `emplacement` and `cover` def rects must differ (else the distinct-tile pin is \
         vacuous)",
    );

    assert_eq!(
        sprite_rect_at(&mut app, emp_cell),
        def_rect(&defs, "emplacement"),
        "the emplacement cell must draw the dedicated `emplacement` tile, NOT the generic cover \
         tile",
    );
    assert_eq!(
        sprite_rect_at(&mut app, cover_cell),
        def_rect(&defs, "cover"),
        "the plain cover cell must still draw the `cover` tile",
    );
}

#[test]
fn occupying_an_emplacement_swaps_its_tile_in_place() {
    let mut app = headless_renderer_app();
    settle_resources(&mut app);

    let l0 = Level::new(0);
    let emp_cell = CellLevel::new(Cell::new(4, 5), l0);

    insert_occupancy(
        &mut app,
        vec![TerrainPlacement::new(emp_cell, TerrainKind::Emplacement)],
    );
    let mut cover_ledger = CoverLedger::new();
    cover_ledger.insert(emp_cell, low_cover_entry());
    app.world_mut().insert_resource(cover_ledger);
    app.world_mut().insert_resource(SurfaceGrid::new());
    app.world_mut().insert_resource(BattleInProgress);

    let emplacement = spawn_emplacement_entity(&mut app, emp_cell);

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
        def_rect(&defs, "emplacement_occupied"),
        def_rect(&defs, "emplacement"),
        "the `emplacement_occupied` def rect must differ from the vacant `emplacement` rect \
         (a real swap)",
    );

    assert_eq!(
        sprite_rect_at(&mut app, emp_cell),
        def_rect(&defs, "emplacement"),
        "an unmanned emplacement's sprite must start on the vacant `emplacement` tile",
    );
    let entity_before = sprite_entity_at(&mut app, emp_cell);
    assert!(
        entity_before.is_some(),
        "the emplacement cell's terrain sprite must exist before it is manned",
    );

    let occupied = app.world_mut().get_mut::<EmplacementState>(emplacement);
    assert!(
        occupied.is_some(),
        "the spawned emplacement entity must carry EmplacementState",
    );
    let Some(mut state) = occupied else { return };
    *state = EmplacementState::Occupied;
    app.update();

    assert_eq!(
        sprite_rect_at(&mut app, emp_cell),
        def_rect(&defs, "emplacement_occupied"),
        "a manned emplacement's sprite must swap to the `emplacement_occupied` tile",
    );
    assert_eq!(
        sprite_entity_at(&mut app, emp_cell),
        entity_before,
        "the manned emplacement cell must be the SAME Entity after the swap (no despawn/respawn)",
    );

    let vacate = app.world_mut().get_mut::<EmplacementState>(emplacement);
    assert!(
        vacate.is_some(),
        "the emplacement entity must still carry EmplacementState",
    );
    let Some(mut state) = vacate else { return };
    *state = EmplacementState::Vacant;
    app.update();
    assert_eq!(
        sprite_rect_at(&mut app, emp_cell),
        def_rect(&defs, "emplacement"),
        "a vacated emplacement's sprite must restore to the vacant `emplacement` tile",
    );
}
