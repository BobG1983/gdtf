//! Headless turn-cycle tests (GTW-309) — the contract's three cases: player End Turn
//! auto-passes the enemy back to the player; the player team's TU resets to max at its
//! turn-start; the enemy team's TU is untouched when the player's turn starts (and vice
//! versa).

use bevy::prelude::{App, Entity, MinimalPlugins, World};

use crate::{
    acts::{EndTurnRequested, SimActsPlugin},
    battle::PlayerFaction,
    cover::CoverLedger,
    ganger::{Faction, Tu, TuMax},
    occupancy::OccupancyGrid,
    rng::{BattleSeed, SimRng},
    surface::SurfaceGrid,
    tuning::CombatTuning,
    turn::{ActiveFaction, regen_team_tu},
};

/// A fixed seed for the per-test RNG stream (arbitrary, not tuned).
const SEED: u64 = 0x5A1C_AC75;

/// Gang `0` is the player team in these fixtures; gang `1` is the enemy.
const PLAYER: Faction = Faction::new(0);
/// The enemy team in these fixtures.
const ENEMY: Faction = Faction::new(1);

/// Build a headless turn-cycle app: [`MinimalPlugins`] + [`SimActsPlugin`] (registers the
/// [`EndTurnRequested`] buffer + the [`dispatch_end_turn`](crate::turn::dispatch_end_turn)
/// system), seeded with the battle-lifetime turn resources ([`ActiveFaction`] on the
/// player, [`PlayerFaction`] = the player). No [`BattleInProgress`](crate::battle::BattleInProgress)
/// is inserted: this harness adds only [`SimActsPlugin`] (not the
/// [`OccupancyMaintenancePlugin`](crate::occupancy_sync::OccupancyMaintenancePlugin) that
/// owns the `SimSystems::Simulate` `configure_sets`), so the set carries no
/// `BattleInProgress` gate here and `dispatch_end_turn`'s own
/// `run_if(resource_exists::<ActiveFaction>)` is the live guard (the `acts` test precedent).
///
/// The shared sim resources the OTHER (ungated, in this harness) dispatch systems read —
/// the three grids, a seeded [`SimRng`], and [`CombatTuning`] — are inserted too (mirroring
/// the `acts` test's `insert_sim_resources`), so those systems' `Res<_>` params validate
/// and the update runs through to `dispatch_end_turn`.
fn turn_app() -> App {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins);
    app.add_plugins(SimActsPlugin);
    app.insert_resource(OccupancyGrid::new());
    app.insert_resource(SurfaceGrid::new());
    app.insert_resource(CoverLedger::new());
    app.insert_resource(SimRng::from_seed(BattleSeed::new(SEED)));
    app.insert_resource(CombatTuning::default());
    app.insert_resource(ActiveFaction::new(PLAYER));
    app.insert_resource(PlayerFaction::new(PLAYER));
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

/// (a) A player End Turn auto-passes the enemy turn straight back to the player — the
/// active faction ends the cycle back on the player.
#[test]
fn player_end_turn_returns_control_to_player() {
    let mut app = turn_app();
    spawn_drained(app.world_mut(), PLAYER, 100);
    spawn_drained(app.world_mut(), ENEMY, 100);

    // Sanity: the player acts first.
    assert_eq!(**app.world().resource::<ActiveFaction>(), PLAYER);

    app.world_mut().write_message(EndTurnRequested);
    app.update();

    // Player ended → enemy → (no AI) auto-pass → player. Control is back on the player.
    assert_eq!(**app.world().resource::<ActiveFaction>(), PLAYER);
}

/// (b) The player team's TU resets to its [`TuMax`] at its turn-start — every player
/// ganger's drained pool is restored after the cycle returns control to the player.
#[test]
fn player_team_tu_resets_to_max_at_turn_start() {
    let mut app = turn_app();
    let player_a = spawn_drained(app.world_mut(), PLAYER, 100);
    let player_b = spawn_drained(app.world_mut(), PLAYER, 80);

    app.world_mut().write_message(EndTurnRequested);
    app.update();

    // Both player gangers' pools are restored to their own TuMax at the player's
    // turn-start (the auto-pass returns control + regens the player team).
    assert_eq!(*tu_of(app.world(), player_a), 100);
    assert_eq!(*tu_of(app.world(), player_b), 80);
}

/// (c) The enemy team's TU is UNTOUCHED when the player's turn starts (and the player's is
/// untouched when only the enemy regens) — TU regenerates for exactly the team whose turn
/// is starting.
#[test]
fn other_team_tu_is_untouched_at_a_teams_turn_start() {
    let mut app = turn_app();
    // The enemy starts with SOME TU it should keep across the player's turn-start.
    let enemy = app
        .world_mut()
        .spawn((ENEMY, Tu::new(33), TuMax::new(100)))
        .id();
    let player = spawn_drained(app.world_mut(), PLAYER, 100);

    app.world_mut().write_message(EndTurnRequested);
    app.update();

    // The cycle regens enemy-then-player; the FINAL player regen does NOT touch the enemy,
    // and the enemy's own regen earlier set it to 100, then it stays — so what matters for
    // the contract is the asymmetry: the player's turn-start regen leaves the enemy as the
    // enemy regen left it, never re-touched by the player pass.
    // Player pool is restored at its turn-start.
    assert_eq!(*tu_of(app.world(), player), 100);

    // Now drive the PURE helper directly to prove the one-team isolation precisely: a
    // player regen leaves the enemy's pool exactly as-is.
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
    // The player team reset to max; the enemy team's pool is UNTOUCHED.
    assert_eq!(*player_tu, 100);
    assert_eq!(*enemy_tu, 7);

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
    assert_eq!(*enemy_tu2, 100);
    assert_eq!(*player_tu2, 42);

    // The earlier app-driven enemy-then-player cycle left the enemy at its OWN max (its
    // turn-start regen ran), confirming the enemy DID regen on its (auto-passed) turn.
    assert_eq!(*tu_of(app.world(), enemy), 100);
}
