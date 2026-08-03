use bevy::prelude::{App, Entity, Messages, MinimalPlugins, World};

use crate::{
    acts::{EndTurnRequested, SimActsPlugin},
    ganger::{Faction, Tu, TuMax},
    rng::BattleSeed,
    test_support::insert_sim_resources,
    turn::{ActiveFaction, TurnStarted, regen_team_tu},
};

const SEED: u64 = 0x5A1C_AC75;

const PLAYER: Faction = Faction::new(0);
const ENEMY: Faction = Faction::new(1);

fn turn_app() -> App {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins);
    app.add_plugins(SimActsPlugin);
    insert_sim_resources(&mut app, BattleSeed::new(SEED));
    app.insert_resource(ActiveFaction::new(PLAYER));
    app
}

fn spawn_drained(world: &mut World, faction: Faction, tu_max: u8) -> Entity {
    world.spawn((faction, Tu::new(0), TuMax::new(tu_max))).id()
}

fn tu_of(world: &World, entity: Entity) -> Tu {
    world.get::<Tu>(entity).copied().unwrap_or_default()
}

fn drain_turns_started(app: &mut App) -> Vec<TurnStarted> {
    app.world_mut()
        .resource_mut::<Messages<TurnStarted>>()
        .drain()
        .collect()
}

#[test]
fn player_end_turn_hands_off_to_enemy_and_stops() {
    let mut app = turn_app();
    spawn_drained(app.world_mut(), PLAYER, 100);
    spawn_drained(app.world_mut(), ENEMY, 100);

    assert_eq!(**app.world().resource::<ActiveFaction>(), PLAYER);

    app.world_mut().write_message(EndTurnRequested);
    app.update();

    assert_eq!(
        **app.world().resource::<ActiveFaction>(),
        ENEMY,
        "a player End Turn hands off to the enemy and stops there; control returns to the player only once the enemy turn ends",
    );
}

#[test]
fn end_turn_emits_one_turn_started_for_the_now_active_team() {
    let mut app = turn_app();
    spawn_drained(app.world_mut(), PLAYER, 100);
    spawn_drained(app.world_mut(), ENEMY, 100);

    app.world_mut().write_message(EndTurnRequested);
    app.update();

    let turns = drain_turns_started(&mut app);
    assert_eq!(
        turns.len(),
        1,
        "a single End Turn now crosses exactly ONE turn boundary (the hand-off), not two: \
         {turns:?}",
    );
    assert_eq!(
        turns.first().map(|t| t.now_active),
        Some(ENEMY),
        "the one TurnStarted announces the enemy's turn (the hand-off)",
    );
}

#[test]
fn now_active_team_regens_its_tu_other_team_untouched() {
    let mut app = turn_app();
    let player = spawn_drained(app.world_mut(), PLAYER, 100);
    let enemy = spawn_drained(app.world_mut(), ENEMY, 80);

    app.world_mut().write_message(EndTurnRequested);
    app.update();

    assert_eq!(
        *tu_of(app.world(), enemy),
        80,
        "the now-active (enemy) team's TU resets to its own TuMax at its turn-start",
    );
    assert_eq!(
        *tu_of(app.world(), player),
        0,
        "the player team's TU is untouched at the enemy's turn-start (no auto-pass regen)",
    );
}

#[test]
fn enemy_turn_returns_to_player_when_the_enemy_is_done() {
    let mut app = turn_app();
    let player = spawn_drained(app.world_mut(), PLAYER, 100);
    spawn_drained(app.world_mut(), ENEMY, 80);

    app.world_mut().write_message(EndTurnRequested);
    app.update();
    app.update();

    assert_eq!(
        **app.world().resource::<ActiveFaction>(),
        PLAYER,
        "the empty enemy turn ends, returning control to the player",
    );
    assert_eq!(
        *tu_of(app.world(), player),
        100,
        "control returning to the player regenerates the player team's TU at its turn-start",
    );
}

#[test]
fn regen_team_tu_resets_only_the_named_team() {
    let mut enemy_tu = Tu::new(7);
    let enemy_faction = ENEMY;
    let enemy_max = TuMax::new(100);
    let mut player_tu = Tu::new(0);
    let player_faction = PLAYER;
    let player_max = TuMax::new(100);
    regen_team_tu(
        [
            (&enemy_faction, &mut enemy_tu, &enemy_max),
            (&player_faction, &mut player_tu, &player_max),
        ],
        PLAYER,
    );
    assert_eq!(*player_tu, 100, "the player team reset to its max");
    assert_eq!(*enemy_tu, 7, "the enemy team's pool is UNTOUCHED");

    let mut enemy_tu2 = Tu::new(0);
    let mut player_tu2 = Tu::new(42);
    regen_team_tu(
        [
            (&enemy_faction, &mut enemy_tu2, &enemy_max),
            (&player_faction, &mut player_tu2, &player_max),
        ],
        ENEMY,
    );
    assert_eq!(*enemy_tu2, 100, "the enemy team reset to its max");
    assert_eq!(*player_tu2, 42, "the player team's pool is UNTOUCHED");
}
