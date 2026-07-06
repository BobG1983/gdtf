//! Headless turn-cycle tests (GTW-309; GTW-70 removed the enemy auto-pass) — the new
//! contract's cases: a player End Turn hands off to the enemy and STOPS (no auto-pass);
//! it emits ONE `TurnStarted` for the now-active team; the now-active team's TU resets to
//! max while the other team's is untouched; and the enemy turn returns to the player once
//! the enemy is done (the GTW-70 brain ends an empty enemy turn). The pure `regen_team_tu`
//! one-team isolation is asserted directly on the helper.

use bevy::prelude::{App, Entity, Messages, MinimalPlugins, World};

use crate::{
    acts::{EndTurnRequested, SimActsPlugin},
    ganger::{Faction, Tu, TuMax},
    rng::BattleSeed,
    test_support::insert_sim_resources,
    turn::{ActiveFaction, TurnStarted, regen_team_tu},
};

/// A fixed seed for the per-test RNG stream (arbitrary, not tuned).
const SEED: u64 = 0x5A1C_AC75;

/// Gang `0` is the player team in these fixtures; gang `1` is the enemy.
const PLAYER: Faction = Faction::new(0);
/// The enemy team in these fixtures.
const ENEMY: Faction = Faction::new(1);

/// Build a headless turn-cycle app: [`MinimalPlugins`] + [`SimActsPlugin`] (registers the
/// [`EndTurnRequested`] buffer + the [`dispatch_end_turn`](crate::turn::dispatch_end_turn)
/// engine AND the GTW-70 [`enemy_ai_turn`](crate::ai::enemy_ai_turn) brain), seeded with the
/// battle-lifetime turn resources ([`ActiveFaction`] on the player, [`PlayerFaction`] = the
/// player). No [`BattleInProgress`](crate::battle::BattleInProgress) is inserted: this
/// harness adds only [`SimActsPlugin`] (not the
/// [`OccupancyMaintenancePlugin`](crate::occupancy_sync::OccupancyMaintenancePlugin) that
/// owns the `SimSystems::Simulate` `configure_sets`), so the set carries no
/// `BattleInProgress` gate here and `dispatch_end_turn`'s / `enemy_ai_turn`'s own
/// `run_if(resource_exists::<ActiveFaction>)` is the live guard (the `acts` test precedent).
///
/// The shared sim resources the OTHER (ungated, in this harness) dispatch systems read —
/// the grids, the five seeded RNG streams (GTW-14), and [`CombatTuning`] — are inserted too,
/// so those systems' `Res<_>` params validate and the update runs through. No
/// `OmniscientFog` is seeded: the bare turn-test gangers carry no [`Position`](crate::ganger::Position),
/// so the brain sees no actable enemy and simply ends an enemy turn — exactly the empty-enemy
/// pass these tests exercise.
fn turn_app() -> App {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins);
    app.add_plugins(SimActsPlugin);
    // The canonical seeding litany (GTW-576) — grids, ledgers, five RNG streams, empty
    // injury content, default tuning + uniform floor costs, PlayerFaction on gang 0
    // (== PLAYER here) — plus the turn engine's ActiveFaction seed.
    insert_sim_resources(&mut app, BattleSeed::new(SEED));
    app.insert_resource(ActiveFaction::new(PLAYER));
    app
}

/// Spawn a ganger of `faction` with a DRAINED pool (`tu = 0`) and a `tu_max` ceiling, so a
/// turn-start regen is observable as `tu` jumping back to `tu_max`.
fn spawn_drained(world: &mut World, faction: Faction, tu_max: u8) -> Entity {
    world.spawn((faction, Tu::new(0), TuMax::new(tu_max))).id()
}

/// Read a ganger's current [`Tu`] from the world.
fn tu_of(world: &World, entity: Entity) -> Tu {
    world.get::<Tu>(entity).copied().unwrap_or_default()
}

/// Drain the buffered [`TurnStarted`] combat-log messages emitted this run, in order
/// (GTW-328).
fn drain_turns_started(app: &mut App) -> Vec<TurnStarted> {
    app.world_mut()
        .resource_mut::<Messages<TurnStarted>>()
        .drain()
        .collect()
}

/// (GTW-70) A player End Turn hands control to the ENEMY and STOPS there — there is no
/// auto-pass back to the player. After one update the active faction is the enemy; the
/// GTW-70 brain (not `dispatch_end_turn`) is what later returns control to the player.
#[test]
fn player_end_turn_hands_off_to_enemy_and_stops() {
    let mut app = turn_app();
    spawn_drained(app.world_mut(), PLAYER, 100);
    spawn_drained(app.world_mut(), ENEMY, 100);

    // Sanity: the player acts first.
    assert_eq!(**app.world().resource::<ActiveFaction>(), PLAYER);

    app.world_mut().write_message(EndTurnRequested);
    app.update();

    // Player ended → enemy, and the cycle STOPS on the enemy (no auto-pass back).
    assert_eq!(
        **app.world().resource::<ActiveFaction>(),
        ENEMY,
        "a player End Turn hands off to the enemy and stops there (GTW-70 removed the \
         auto-pass); control returns to the player only once the enemy turn ends",
    );
}

/// (GTW-70) A player End Turn emits EXACTLY ONE [`TurnStarted`] — the enemy turn start (the
/// hand-off) — and no more, since the cycle no longer double-advances back to the player in
/// the same request. This pins the per-advance combat-log boundary signal under the new
/// single-advance contract.
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

/// (GTW-70) TU regenerates for exactly the team whose turn is STARTING — the now-active
/// (enemy) team's drained pool resets to its [`TuMax`], while the player team's pool is left
/// untouched (it regenerates only when control returns to it). The asymmetry is the contract.
#[test]
fn now_active_team_regens_its_tu_other_team_untouched() {
    let mut app = turn_app();
    let player = spawn_drained(app.world_mut(), PLAYER, 100);
    let enemy = spawn_drained(app.world_mut(), ENEMY, 80);

    app.world_mut().write_message(EndTurnRequested);
    app.update();

    // The enemy's turn started → its pool is restored to its own TuMax.
    assert_eq!(
        *tu_of(app.world(), enemy),
        80,
        "the now-active (enemy) team's TU resets to its own TuMax at its turn-start",
    );
    // The player's pool is NOT touched — it regenerates only at its own next turn-start.
    assert_eq!(
        *tu_of(app.world(), player),
        0,
        "the player team's TU is untouched at the enemy's turn-start (no auto-pass regen)",
    );
}

/// (GTW-70) The enemy turn RETURNS to the player once the enemy is done — with no actable
/// enemy ganger (these bare fixtures carry no `Position`), the GTW-70 brain immediately ends
/// the empty enemy turn, so a second update hands control back to the player and regenerates
/// the player's TU. This proves the brain — not `dispatch_end_turn` — now closes the cycle.
#[test]
fn enemy_turn_returns_to_player_when_the_enemy_is_done() {
    let mut app = turn_app();
    let player = spawn_drained(app.world_mut(), PLAYER, 100);
    spawn_drained(app.world_mut(), ENEMY, 80);

    app.world_mut().write_message(EndTurnRequested);
    // Update 1: player → enemy (the brain emits its own EndTurnRequested for the empty
    // enemy turn). Update 2: that EndTurnRequested cycles enemy → player.
    app.update();
    app.update();

    assert_eq!(
        **app.world().resource::<ActiveFaction>(),
        PLAYER,
        "the empty enemy turn ends via the GTW-70 brain, returning control to the player",
    );
    assert_eq!(
        *tu_of(app.world(), player),
        100,
        "control returning to the player regenerates the player team's TU at its turn-start",
    );
}

/// The pure [`regen_team_tu`] helper resets ONLY the named team — a player regen leaves the
/// enemy's pool exactly as-is, and the symmetric enemy regen leaves the player's untouched.
/// Driven directly on the helper (no `App`), the one-team isolation the cycle relies on.
#[test]
fn regen_team_tu_resets_only_the_named_team() {
    // A player regen leaves the enemy's pool exactly as-is.
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

    // ... and the symmetric direction: an enemy regen leaves the player untouched.
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
