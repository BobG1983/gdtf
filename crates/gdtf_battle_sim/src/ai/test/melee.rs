use bevy::prelude::{App, Entity, Messages, World};

use super::support::{
    ENEMY, PLAYER, STRIKE_TU, brain_app, drain_fires, drain_melees, drain_moves,
    drive_until_player_turn, give_melee, ground, melee_bundle, place_occupant, spawn_combatant,
    tu_of,
};
use crate::{
    acts::{MeleeRequested, MeleeStruck},
    armor::{ArmorHardness, ArmorProtection},
    cover::{CoverEntry, CoverHp, CoverLedger, HeightBand},
    ganger::{
        Aiming, Direction, Facing, Faction, Fight, Hp, LifeState, Luck, Position, Shooting, Stance,
        StanceKind, Toughness, Tu, TuMax, Wounds,
    },
    inflicted_wound::InflictedWounds,
    metric::{Cell, CellLevel},
    terrain::entity::TerrainPieceKind,
    weapon::WieldedBy,
};

fn spawn_melee_combatant(
    world: &mut World,
    at: CellLevel,
    faction: Faction,
    facing: Direction,
    tu: u8,
) -> Entity {
    let ganger = world
        .spawn((
            Position::new(at),
            Facing::new(facing),
            Stance::new(StanceKind::Standing),
            Aiming::new(false),
            Shooting::new(1.0),
            Fight::new(1.0),
            Tu::new(tu),
            TuMax::new(tu),
            faction,
            (
                Hp::new(50),
                Wounds::new(10),
                LifeState::Alive,
                InflictedWounds::default(),
                Toughness::new(1.0),
                Luck::new(0.0),
            ),
        ))
        .id();
    world.spawn((WieldedBy::new(ganger), melee_bundle()));
    ganger
}

fn wall_the_diagonal_corners(app: &mut App) {
    let Some(mut ledger) = app.world_mut().get_resource_mut::<CoverLedger>() else {
        unreachable!("the sim resources carry a cover ledger");
    };
    for at in [ground(6, 5), ground(5, 6)] {
        ledger.insert(
            at,
            CoverEntry::seeded(
                CoverHp::new(100),
                HeightBand::High,
                ArmorProtection::new(50),
                ArmorHardness::new(50),
                TerrainPieceKind::Wall,
            ),
        );
    }
}

fn drain_struck(app: &mut App) -> Vec<MeleeStruck> {
    app.world_mut()
        .resource_mut::<Messages<MeleeStruck>>()
        .drain()
        .collect()
}

fn drive_until_player(app: &mut App) -> (Vec<MeleeRequested>, Vec<MeleeStruck>) {
    let mut melees = Vec::new();
    let mut struck = Vec::new();
    drive_until_player_turn(app, |app| {
        melees.extend(drain_melees(app));
        struck.extend(drain_struck(app));
    });
    (melees, struck)
}

#[test]
fn melee_only_adjacent_swings_and_dispatch_runs_same_frame() {
    let mut app = brain_app();
    let player_at = ground(6, 5);
    let enemy = spawn_melee_combatant(app.world_mut(), ground(5, 5), ENEMY, Direction::East, 40);
    let player = spawn_combatant(app.world_mut(), player_at, PLAYER, Direction::West, 100, 2);
    app.world_mut().entity_mut(player).insert(Fight::new(0.0));
    place_occupant(&mut app, player_at, player);

    app.update();
    let melees = drain_melees(&mut app);
    let struck = drain_struck(&mut app);

    assert_eq!(
        melees,
        vec![MeleeRequested::new(enemy, player)],
        "a melee-only enemy beside a living foe must emit MeleeRequested for that foe",
    );
    assert!(
        struck
            .iter()
            .any(|hit| hit.attacker == enemy && hit.target == player),
        "dispatch_melee must resolve that request on the same frame: {struck:?}",
    );
    assert!(
        tu_of(&app, enemy) < 40,
        "the real dispatch must spend strike TU: {}",
        tu_of(&app, enemy),
    );
}

#[test]
fn gun_with_a_valid_shot_fires_instead_of_swinging() {
    let mut app = brain_app();
    let player_at = ground(6, 5);
    let enemy = spawn_combatant(
        app.world_mut(),
        ground(5, 5),
        ENEMY,
        Direction::East,
        100,
        6,
    );
    give_melee(app.world_mut(), enemy);
    let player = spawn_combatant(app.world_mut(), player_at, PLAYER, Direction::West, 100, 2);
    place_occupant(&mut app, player_at, player);

    let mut fires = Vec::new();
    let mut melees = Vec::new();
    drive_until_player_turn(&mut app, |app| {
        fires.extend(drain_fires(app));
        melees.extend(drain_melees(app));
    });

    assert!(
        fires.iter().any(|fire| fire.shooter == enemy
            && fire.target_cell == Cell::new(6, 5)
            && *fire.target_level == 0),
        "a loaded gun that can see the adjacent player must fire: {fires:?}",
    );
    assert!(
        melees.is_empty(),
        "fire is preferred over melee when a shot is available: {melees:?}",
    );
}

#[test]
fn unaffordable_melee_does_not_emit_and_the_turn_ends() {
    let mut app = brain_app();
    let player_at = ground(6, 5);
    let _enemy = spawn_melee_combatant(
        app.world_mut(),
        ground(5, 5),
        ENEMY,
        Direction::East,
        STRIKE_TU.saturating_sub(1),
    );
    let player = spawn_combatant(app.world_mut(), player_at, PLAYER, Direction::West, 100, 2);
    place_occupant(&mut app, player_at, player);

    let (melees, _) = drive_until_player(&mut app);
    assert!(
        melees.is_empty(),
        "a swing the pool cannot cover must not emit MeleeRequested: {melees:?}",
    );
}

#[test]
fn melee_out_of_reach_still_advances() {
    let mut app = brain_app();
    let player_at = ground(40, 5);
    let enemy = spawn_melee_combatant(app.world_mut(), ground(2, 5), ENEMY, Direction::East, 20);
    let player = spawn_combatant(app.world_mut(), player_at, PLAYER, Direction::West, 100, 6);
    place_occupant(&mut app, player_at, player);

    let mut moves = Vec::new();
    let mut melees = Vec::new();
    drive_until_player_turn(&mut app, |app| {
        moves.extend(drain_moves(app));
        melees.extend(drain_melees(app));
    });

    assert!(
        melees.is_empty(),
        "a melee-only enemy out of reach must not swing: {melees:?}",
    );
    assert!(
        moves
            .iter()
            .any(|step| step.actor == enemy && step.dest.x > 2),
        "out of reach they still advance toward the player: {moves:?}",
    );
}

#[test]
fn los_blocked_adjacency_does_not_swing_and_the_turn_ends() {
    let mut app = brain_app();
    let player_at = ground(6, 6);
    let _enemy = spawn_melee_combatant(app.world_mut(), ground(5, 5), ENEMY, Direction::East, 40);
    let player = spawn_combatant(app.world_mut(), player_at, PLAYER, Direction::West, 100, 2);
    place_occupant(&mut app, player_at, player);
    wall_the_diagonal_corners(&mut app);

    let (melees, struck) = drive_until_player(&mut app);
    assert!(
        melees.is_empty(),
        "a diagonal wall corner must block the swing: {melees:?}",
    );
    assert!(
        struck.is_empty(),
        "dispatch must not resolve a blocked swing: {struck:?}",
    );
}
