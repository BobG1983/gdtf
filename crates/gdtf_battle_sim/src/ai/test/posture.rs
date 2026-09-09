use bevy::prelude::{App, Entity};

use super::support::{
    ENEMY, PLAYER, aiming_of, brain_app, drain_aims, drain_fires, drain_stances,
    drive_until_player_turn, ground, place_occupant, spawn_combatant, stance_of, tu_of,
};
use crate::{
    acts::{AimRequest, FireRequested, SetAimingRequested, SetStanceRequested},
    armor::{ArmorHardness, ArmorProtection},
    cover::{CoverEntry, CoverHp, CoverLedger, HeightBand},
    ganger::{Direction, StanceKind, Suppressed, Tu},
    metric::{Cell, CellLevel},
    terrain::entity::TerrainPieceKind,
    turn::ActiveFaction,
    weapon::{FireModeSpec, ModeConeMult, ModeKind, ModeShots, ModeTuPercent},
};

fn single_shot() -> FireModeSpec {
    FireModeSpec::new(
        ModeKind::Single,
        ModeConeMult::new(1.0),
        ModeTuPercent::new(0.2),
        ModeShots::new(1),
    )
}

fn pin_by_incoming_fire(app: &mut App, shooter: Entity, at: CellLevel) {
    app.insert_resource(ActiveFaction::new(PLAYER));
    app.world_mut().write_message(FireRequested::new(
        shooter,
        single_shot(),
        at.cell(),
        at.level(),
    ));
    app.update();
    app.insert_resource(ActiveFaction::new(ENEMY));
}

fn wall_toward(app: &mut App, at: CellLevel) {
    let Some(mut ledger) = app.world_mut().get_resource_mut::<CoverLedger>() else {
        unreachable!("the sim resources carry a cover ledger");
    };
    ledger.insert(
        at,
        CoverEntry::seeded(
            CoverHp::new(10),
            HeightBand::Mid,
            ArmorProtection::new(1),
            ArmorHardness::new(1),
            TerrainPieceKind::Wall,
        ),
    );
}

fn drive_until_player(app: &mut App) -> Drive {
    let mut aims = Vec::new();
    let mut fires = Vec::new();
    let mut stances = Vec::new();
    drive_until_player_turn(app, |app| {
        aims.extend(drain_aims(app));
        fires.extend(drain_fires(app));
        stances.extend(drain_stances(app));
    });
    Drive {
        aims,
        fires,
        stances,
    }
}

struct Drive {
    aims:    Vec<SetAimingRequested>,
    fires:   Vec<FireRequested>,
    stances: Vec<SetStanceRequested>,
}

#[test]
fn aim_then_fire_aimed_when_the_premium_fits() {
    let mut app = brain_app();
    let player_at = ground(8, 5);
    let enemy = spawn_combatant(
        app.world_mut(),
        ground(2, 5),
        ENEMY,
        Direction::East,
        100,
        6,
    );
    let player = spawn_combatant(app.world_mut(), player_at, PLAYER, Direction::West, 100, 2);
    place_occupant(&mut app, player_at, player);

    app.update();
    let aims = drain_aims(&mut app);
    let fires = drain_fires(&mut app);
    assert_eq!(
        aims,
        vec![SetAimingRequested::new(enemy, AimRequest::new(true))],
        "an engageable target and an affordable aimed shot must aim first",
    );
    assert!(
        fires.is_empty(),
        "aiming uses the one-act step, so the shot waits: {fires:?}",
    );
    assert!(
        aiming_of(&app, enemy),
        "the real set-aiming dispatch must flip Aiming on the same frame",
    );
    assert_eq!(
        tu_of(&app, enemy),
        100,
        "aiming is free: {}",
        tu_of(&app, enemy),
    );

    let driven = drive_until_player(&mut app);
    assert!(
        driven.fires.iter().any(|fire| fire.shooter == enemy
            && fire.target_cell == Cell::new(8, 5)
            && *fire.target_level == 0),
        "the next step fires the aimed shot at the visible player: {fires:?}",
    );
    assert!(
        aiming_of(&app, enemy),
        "the shot is the aimed one: Aiming stays on",
    );
}

#[test]
fn unaimed_shot_when_the_aim_premium_does_not_fit() {
    let mut app = brain_app();
    let player_at = ground(8, 5);
    let enemy = spawn_combatant(
        app.world_mut(),
        ground(2, 5),
        ENEMY,
        Direction::East,
        100,
        6,
    );
    app.world_mut().entity_mut(enemy).insert(Tu::new(25));
    let player = spawn_combatant(app.world_mut(), player_at, PLAYER, Direction::West, 100, 2);
    place_occupant(&mut app, player_at, player);

    let driven = drive_until_player(&mut app);
    assert!(
        driven.aims.is_empty(),
        "a 25-TU pool cannot cover the aimed premium on a 100-TU max, so they must not aim: {:?}",
        driven.aims,
    );
    assert!(
        driven.fires.iter().any(|fire| fire.shooter == enemy
            && fire.target_cell == Cell::new(8, 5)
            && *fire.target_level == 0),
        "the unaimed shot still fires: {:?}",
        driven.fires,
    );
    assert!(
        !aiming_of(&app, enemy),
        "Aiming stays off when they fire unaimed",
    );
}

#[test]
fn incoming_fire_with_no_cover_crouches() {
    let mut app = brain_app();
    let player_at = ground(40, 5);
    let enemy_at = ground(2, 5);
    let enemy = spawn_combatant(app.world_mut(), enemy_at, ENEMY, Direction::East, 20, 6);
    let player = spawn_combatant(app.world_mut(), player_at, PLAYER, Direction::West, 100, 6);
    place_occupant(&mut app, player_at, player);
    place_occupant(&mut app, enemy_at, enemy);

    pin_by_incoming_fire(&mut app, player, enemy_at);
    assert!(
        app.world().get::<Suppressed>(enemy).is_some(),
        "the real fire path must pin the enemy",
    );
    assert_eq!(
        stance_of(&app, enemy),
        Some(StanceKind::Standing),
        "no cover toward the shot, so auto-stance must leave them standing",
    );

    let driven = drive_until_player(&mut app);
    assert!(
        driven
            .stances
            .iter()
            .any(|stance| stance.actor == enemy && stance.stance == StanceKind::Crouching),
        "a pinned standing enemy with no legal step must crouch: {:?}",
        driven.stances,
    );
    assert_eq!(
        stance_of(&app, enemy),
        Some(StanceKind::Crouching),
        "the real set-stance dispatch must kneel them",
    );
}

#[test]
fn incoming_fire_with_adjacent_cover_does_not_re_crouch() {
    let mut app = brain_app();
    let player_at = ground(40, 5);
    let enemy_at = ground(2, 5);
    let enemy = spawn_combatant(app.world_mut(), enemy_at, ENEMY, Direction::East, 20, 6);
    let player = spawn_combatant(app.world_mut(), player_at, PLAYER, Direction::West, 100, 6);
    place_occupant(&mut app, player_at, player);
    place_occupant(&mut app, enemy_at, enemy);
    wall_toward(&mut app, ground(3, 5));

    pin_by_incoming_fire(&mut app, player, enemy_at);
    assert_eq!(
        stance_of(&app, enemy),
        Some(StanceKind::Crouching),
        "cover toward the shot must auto-stance them to a crouch",
    );

    let driven = drive_until_player(&mut app);
    assert!(
        driven.stances.is_empty(),
        "already crouched by auto-stance, so the brain must not emit SetStanceRequested: {:?}",
        driven.stances,
    );
    assert_eq!(
        stance_of(&app, enemy),
        Some(StanceKind::Crouching),
        "they stay crouched",
    );
}

#[test]
fn idle_crouches_once_then_the_turn_ends() {
    let mut app = brain_app();
    let enemy = spawn_combatant(app.world_mut(), ground(2, 5), ENEMY, Direction::East, 40, 6);

    app.update();
    let stances = drain_stances(&mut app);
    assert_eq!(
        stances,
        vec![SetStanceRequested::new(enemy, StanceKind::Crouching)],
        "an idle standing enemy must crouch instead of standing in the open",
    );
    assert_eq!(
        stance_of(&app, enemy),
        Some(StanceKind::Crouching),
        "the real set-stance dispatch must kneel them on that frame",
    );
    assert!(
        tu_of(&app, enemy) < 40,
        "the crouch spends stance TU: {}",
        tu_of(&app, enemy),
    );

    let mut more = Vec::new();
    drive_until_player_turn(&mut app, |app| more.extend(drain_stances(app)));
    assert!(more.is_empty(), "already crouched, they hold: {more:?}");
}
