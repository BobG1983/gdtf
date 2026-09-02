use std::time::Duration;

use bevy::{
    app::App,
    ecs::{entity::Entity, message::Messages},
    math::Vec3,
    time::TimeUpdateStrategy,
    transform::components::Transform,
};
use gdtf_battle_presenter::{DrawnPosition, GangerSprite, Layer, cell_to_world_layered};
use gdtf_battle_sim::{
    battle::BattleReady,
    cover::CoverLedger,
    emplacement::{EmplacementState, SetEmplacement},
    entity::TerrainCell,
    ganger::Position,
    occupancy::{TerrainKind, TerrainPlacement},
    piece::TerrainGraphicKey,
    prelude::{BattleInProgress, Cell, CellLevel, Level},
    surface::SurfaceGrid,
};

use super::harness::*;

// Longer than the 0.2s ganger tween, so one update settles a retargeted sprite.
const PAST_TWEEN: Duration = Duration::from_millis(400);

fn spawn_emplacement_entity(app: &mut App, key: CellLevel) -> Entity {
    app.world_mut()
        .spawn((
            TerrainCell::new(key),
            TerrainGraphicKey::new("emplacement".to_owned()),
            EmplacementState::Vacant,
        ))
        .id()
}

fn ganger_sprite_translation(app: &mut App, ganger: Entity) -> Option<Vec3> {
    let mut q = app.world_mut().query::<(&GangerSprite, &Transform)>();
    q.iter(app.world())
        .find(|(sprite, _)| sprite.entity == ganger)
        .map(|(_, transform)| transform.translation)
}

fn drawn_position_of(app: &App, ganger: Entity) -> Option<CellLevel> {
    app.world()
        .get::<DrawnPosition>(ganger)
        .map(|drawn| *drawn.position())
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
fn an_occupied_emplacement_draws_the_occupant_and_keeps_its_own_tile() {
    let mut app = headless_renderer_app();
    settle_resources(&mut app);

    let l0 = Level::new(0);
    let seat = CellLevel::new(Cell::new(4, 5), l0);
    let entered_from = CellLevel::new(Cell::new(4, 6), l0);

    insert_occupancy(
        &mut app,
        vec![TerrainPlacement::new(seat, TerrainKind::Emplacement)],
    );
    let mut cover_ledger = CoverLedger::new();
    cover_ledger.insert(seat, low_cover_entry());
    app.world_mut().insert_resource(cover_ledger);
    app.world_mut().insert_resource(SurfaceGrid::new());
    app.world_mut().insert_resource(BattleInProgress);

    let emplacement = spawn_emplacement_entity(&mut app, seat);
    // Spawned before the occupy, so the sprite and the drawn mirror start at this cell.
    let ganger = spawn_test_ganger(&mut app, entered_from);

    app.world_mut()
        .resource_mut::<Messages<BattleReady>>()
        .write(BattleReady);
    app.update();
    app.update();

    let defs = sprite_defs(&app);
    assert!(
        defs.is_some(),
        "the SpriteDefRegistry must be resident after settle"
    );
    let Some(defs) = defs else { return };

    let vacant_rect = sprite_rect_at(&mut app, seat);
    assert_eq!(
        vacant_rect,
        def_rect(&defs, "emplacement"),
        "the seat draws the vacant `emplacement` tile before it is manned",
    );
    assert_eq!(
        drawn_position_of(&app, ganger),
        Some(entered_from),
        "the occupant's DrawnPosition must be seeded at the entered-from cell before the \
         occupy, or the played move below cannot be told from the seed",
    );
    assert_eq!(
        ganger_sprite_translation(&mut app, ganger),
        Some(cell_to_world_layered(
            entered_from.cell(),
            entered_from.level(),
            Layer::Actor
        )),
        "the occupant's sprite must start at the entered-from cell",
    );

    app.world_mut()
        .resource_mut::<Messages<SetEmplacement>>()
        .write(SetEmplacement::occupy(emplacement, ganger));
    app.update();

    assert_eq!(
        app.world().get::<Position>(ganger).copied(),
        Some(Position::new(seat)),
        "apply_emplacement_toggle must move the occupant's sim Position onto the seat",
    );

    let moved = moved_to_log(&mut app, ganger, seat);
    play_past(&mut app, moved, |_| {});

    app.insert_resource(TimeUpdateStrategy::ManualDuration(PAST_TWEEN));
    app.update();
    app.update();

    assert_eq!(
        ganger_sprite_translation(&mut app, ganger),
        Some(cell_to_world_layered(
            seat.cell(),
            seat.level(),
            Layer::Actor
        )),
        "the occupant draws itself ON the seat once the cursor plays its move",
    );
    assert_eq!(
        sprite_rect_at(&mut app, seat),
        vacant_rect,
        "the seat's own tile is unchanged by being manned: there is no occupied-emplacement \
         art any more",
    );
}
