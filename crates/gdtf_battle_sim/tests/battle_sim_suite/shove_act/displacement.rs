use bevy::{app::App, prelude::Entity};
use gdtf_battle_sim::{
    acts::{ShoveOutcome, resolve_shove},
    falls::FallOccurred,
    prelude::{Cell, Direction, OccupancyGrid, Position},
    surface::{SlabState, SurfaceGrid},
};

use super::harness::*;

fn fall_signals(app: &App) -> Vec<FallOccurred> {
    app.world()
        .get_resource::<FallLog>()
        .map(|log| log.falls.clone())
        .unwrap_or_default()
}

#[test]
fn shove_pushes_target_one_cell_away_from_shover() {
    let mut app = shove_app();
    app.insert_resource(SurfaceGrid::new());
    app.insert_resource(OccupancyGrid::new());
    let shover = shove_ganger(app.world_mut(), ground(5, 5), 0);
    let target = shove_ganger(app.world_mut(), ground(6, 5), 1);
    app.update();

    let outcome = resolve_shove(
        Position::new(ground(5, 5)),
        Position::new(ground(6, 5)),
        target,
        app.world().resource::<SurfaceGrid>(),
        app.world().resource::<OccupancyGrid>(),
    );
    assert_eq!(
        outcome,
        ShoveOutcome::Moved { dest: ground(7, 5) },
        "the pure verb pushes the target one cell East (away from the shover), onto the ground"
    );

    shove_and_settle(&mut app, shover, target);

    assert_eq!(
        pos_of(&app, target),
        Some(ground(7, 5)),
        "the deliberate shove pushes the target one cell directly away from the shover"
    );
    assert_eq!(
        pos_of(&app, shover),
        Some(ground(5, 5)),
        "the shover stays put"
    );
}

#[test]
fn shove_off_a_ledge_falls_via_523_and_fires_falloccurred() {
    let mut app = shove_app();
    let mut surface = SurfaceGrid::new();
    surface.set_slab(upper(6, 5, 2), SlabState::Present);
    app.insert_resource(surface);
    app.insert_resource(OccupancyGrid::new());
    let shover = shove_ganger(app.world_mut(), upper(5, 5, 2), 0);
    let target = shove_ganger(app.world_mut(), upper(6, 5, 2), 1);
    app.update();
    let hp_before = hp_of(&app, target);

    shove_and_settle(&mut app, shover, target);

    let Some(landed) = pos_of(&app, target) else {
        unreachable!("the target persists");
    };
    assert_eq!(
        landed.x, 7,
        "the target was pushed East onto the ledge cell"
    );
    assert_eq!(landed.y, 5, "the shove is a lateral (same-y) East push");
    assert!(
        landed.z < 2,
        "the target FELL off the unsupported ledge to a storey below 2 (got z={})",
        landed.z
    );
    let signals = fall_signals(&app);
    assert!(
        signals.iter().any(|s| s.ganger == target),
        "a shove off a ledge fires FallOccurred for the target (the shared 523 fall path)"
    );
    assert!(
        hp_of(&app, target) < hp_before,
        "the fall drops the shoved target's Hp (the shared 523 fall damage)"
    );
}

#[test]
fn shove_onto_supported_cell_moves_without_falling() {
    let mut app = shove_app();
    let mut surface = SurfaceGrid::new();
    surface.set_slab(upper(6, 5, 2), SlabState::Present);
    surface.set_slab(upper(7, 5, 2), SlabState::Present);
    app.insert_resource(surface);
    app.insert_resource(OccupancyGrid::new());
    let shover = shove_ganger(app.world_mut(), upper(5, 5, 2), 0);
    let target = shove_ganger(app.world_mut(), upper(6, 5, 2), 1);
    app.update();
    let hp_before = hp_of(&app, target);

    shove_and_settle(&mut app, shover, target);

    assert_eq!(
        pos_of(&app, target),
        Some(upper(7, 5, 2)),
        "a shove onto a supported (Present-slab) cell moves the target one cell, same storey"
    );
    assert!(
        fall_signals(&app).iter().all(|s| s.ganger != target),
        "a supported shove fires NO FallOccurred (no fall)"
    );
    assert_eq!(
        hp_of(&app, target),
        hp_before,
        "a supported shove deals NO damage (pure displacement — the shove itself never wounds)"
    );
}

#[test]
fn shove_into_an_occupied_cell_is_a_noop() {
    let mut app = shove_app();
    app.insert_resource(SurfaceGrid::new());
    app.insert_resource(OccupancyGrid::new());
    let shover = shove_ganger(app.world_mut(), ground(5, 5), 0);
    let target = shove_ganger(app.world_mut(), ground(6, 5), 1);
    let _blocker = shove_ganger(app.world_mut(), ground(7, 5), 1);
    app.update();
    app.update();

    shove_and_settle(&mut app, shover, target);

    assert_eq!(
        pos_of(&app, target),
        Some(ground(6, 5)),
        "a shove into a ganger-occupied cell is a NO-OP (never shove into a solid)"
    );
    assert!(
        fall_signals(&app).iter().all(|s| s.ganger != target),
        "a blocked shove fires no fall"
    );
}

#[test]
fn pure_verb_diagonal_pushes_one_cell_on_both_axes() {
    let surface = SurfaceGrid::new();
    let occupancy = OccupancyGrid::new();
    let outcome = resolve_shove(
        Position::new(ground(5, 5)),
        Position::new(ground(6, 6)),
        Entity::PLACEHOLDER,
        &surface,
        &occupancy,
    );
    assert_eq!(
        outcome,
        ShoveOutcome::Moved { dest: ground(7, 7) },
        "a diagonal shove pushes one cell on both axes directly away from the shover"
    );
    assert_eq!(
        Direction::from_cells(Cell::new(5, 5), Cell::new(6, 6)),
        Some(Direction::SouthEast),
        "the shove direction is the attacker->target compass step"
    );
}
