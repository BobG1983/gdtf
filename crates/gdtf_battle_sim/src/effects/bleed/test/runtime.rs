//! GTW-336 — the bleed-out clock wired into the LIVE runtime turn cycle.
//!
//! Where the AC1/AC2 tests drive the pure `tick_bleed` system in isolation (the
//! `bleed_app` harness adds it unconditionally), these tests exercise the REAL
//! production path: [`SimActsPlugin`](crate::acts::SimActsPlugin) registers the
//! bleed-out clock so an [`EndTurnRequested`](crate::acts::EndTurnRequested) flows
//! through the turn-cycle engine ([`dispatch_end_turn`](crate::turn::dispatch_end_turn))
//! and — at the enemy-phase start, gated on
//! [`enemy_phase_started`](crate::effects::bleed::enemy_phase_started) — fires `tick_bleed` once
//! per full round (`docs/combat/resolution.md` §9). The buffer
//! [`Bleeding`](crate::effects::bleed::Bleeding) the drain emits is the same one the presenter's
//! consequence FCT reads; these tests prove it is actually produced in a live battle.

use bevy::prelude::{App, Entity, Messages, MinimalPlugins};

use crate::{
    acts::EndTurnRequested,
    battle::{BattleInProgress, BattleRoster, BattleSimPlugin},
    effects::bleed::Bleeding,
    ganger::{Faction, LifeState, Stabilized, Wounds},
    rng::BattleSeed,
    test_support::{GangerEntityBuilder, insert_sim_resources},
    tuning::CombatTuning,
    turn::ActiveFaction,
};

/// Seed the shared battle-lifetime resources a live battle has (everything BUT the
/// [`BattleInProgress`] gate witness): the three grids, the five seeded RNG streams (GTW-14),
/// [`CombatTuning::default`] (persistent `Load`-tier in production), the
/// [`BattleRoster`] the `check_outcome` census reads, and [`PlayerFaction`] +
/// [`ActiveFaction`] on the player (the player acts first). [`BattleSimPlugin`] (added
/// by the caller) bundles
/// [`OccupancyMaintenancePlugin`](crate::occupancy_sync::OccupancyMaintenancePlugin) and
/// [`SimActsPlugin`](crate::acts::SimActsPlugin) and installs the real `BattleInProgress`
/// gate on the `SimSystems::Simulate` band — so these tests drive the exact production
/// registration, gate included.
fn seed_battle_resources(app: &mut App) {
    // The canonical seeding litany (GTW-576) — grids, ledgers, five RNG streams, empty
    // injury content, default tuning + uniform floor costs, PlayerFaction on gang 0
    // (== PLAYER here).
    insert_sim_resources(app, BattleSeed::new(SEED));
    // The live-runtime extras on top of the litany: the BattleRoster the `check_outcome`
    // census reads, and ActiveFaction on the player (the player acts first).
    app.insert_resource(BattleRoster::new([PLAYER, ENEMY]));
    app.insert_resource(ActiveFaction::new(PLAYER));
}

/// A fixed seed for the per-test RNG stream (arbitrary, not tuned).
const SEED: u64 = 0x5A1C_AC75;

/// Gang `0` is the player team; gang `1` is the enemy.
const PLAYER: Faction = Faction::new(0);
/// The enemy team.
const ENEMY: Faction = Faction::new(1);

/// Build the FULL live-runtime harness: [`MinimalPlugins`] + the real production
/// [`BattleSimPlugin`] (which bundles
/// [`OccupancyMaintenancePlugin`](crate::occupancy_sync::OccupancyMaintenancePlugin) +
/// [`SimActsPlugin`](crate::acts::SimActsPlugin) — the turn-cycle engine + the GTW-336
/// bleed wiring — AND installs the `run_if(resource_exists::<BattleInProgress>)` gate on
/// the `SimSystems::Simulate` band), seeded with the battle-lifetime resources a live
/// battle has PLUS the [`BattleInProgress`] gate witness (so the gated Simulate band
/// actually runs). Driving through `BattleSimPlugin` means these tests exercise the SAME
/// registration the live runtime uses — gate included — not a hand-rolled subset.
fn live_app() -> App {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins);
    app.add_plugins(BattleSimPlugin);
    seed_battle_resources(&mut app);
    app.insert_resource(BattleInProgress);
    app
}

/// The bleed rate the default [`CombatTuning`] carries — read, never pinned; tests assert
/// the per-round drop EQUALS this (a relation to the tuning value, not a fixed magnitude).
fn bleed_rate() -> u8 {
    *CombatTuning::default().bleed_rate
}

/// Spawn a ganger of `faction` in the given [`LifeState`] with a Wounds pool, plus a full
/// TU pool (so the turn engine's regen has something to act on, matching a real ganger).
fn bleeding_ganger(app: &mut App, faction: Faction, life: LifeState, wounds: u8) -> Entity {
    GangerEntityBuilder::new()
        .faction(faction)
        .life_state(life)
        .wounds(wounds)
        .tu(0)
        .tu_max(100)
        .spawn(app.world_mut())
}

/// Read a ganger's current Wounds back out (0 if somehow absent — no `unwrap`).
fn wounds_of(app: &App, ganger: Entity) -> u8 {
    app.world().get::<Wounds>(ganger).map_or(0, |w| **w)
}

/// Read a ganger's current [`LifeState`] back out (Alive if absent — no `unwrap`).
fn life_of(app: &App, ganger: Entity) -> LifeState {
    app.world()
        .get::<LifeState>(ganger)
        .copied()
        .unwrap_or(LifeState::Alive)
}

/// Drain the buffered [`Bleeding`] messages emitted this run, in order.
fn drain_bleeding(app: &mut App) -> Vec<Bleeding> {
    app.world_mut()
        .resource_mut::<Messages<Bleeding>>()
        .drain()
        .collect()
}

/// Send one End-Turn request and run the cycle (player → enemy → auto-pass → player).
fn end_turn(app: &mut App) {
    app.world_mut().write_message(EndTurnRequested);
    app.update();
}

/// GTW-336 core — a player End Turn (one full round) drains a live Downed ganger's Wounds
/// by exactly `bleed_rate` AND emits a `Bleeding` carrying it. This is the wiring under
/// test: the drain only runs because `dispatch_end_turn` crossed the enemy-phase boundary
/// and `enemy_phase_started` gated `tick_bleed` on. Before this slice nothing registered
/// the system, so this assertion would fail (Wounds unchanged, no `Bleeding`).
#[test]
fn a_full_round_bleeds_a_live_downed_ganger() {
    let rate = bleed_rate();
    // A pool well above the rate so the round is non-lethal (measuring the per-round drop).
    let start = rate.saturating_add(10);

    let mut app = live_app();
    let downed = bleeding_ganger(&mut app, PLAYER, LifeState::Downed, start);

    end_turn(&mut app);

    assert_eq!(
        wounds_of(&app, downed),
        start - rate,
        "one full round (a player End Turn) must drain exactly bleed_rate from a live \
         Downed ganger — the bleed-out clock is wired into the turn cycle (GTW-336)",
    );
    assert_eq!(
        life_of(&app, downed),
        LifeState::Downed,
        "a non-lethal round leaves the ganger Downed",
    );

    let bled = drain_bleeding(&mut app);
    assert_eq!(
        bled.iter().filter(|b| b.ganger == downed).count(),
        1,
        "the round must emit exactly one Bleeding for the Downed ganger (the presenter's \
         FCT pop drains this buffer): {bled:?}",
    );
}

/// GTW-336 once-per-round — the clock ticks ONCE per full round (the enemy-phase start),
/// not once per `TurnStarted`. A player End Turn crosses TWO turn boundaries (enemy start +
/// player auto-pass), but only the enemy one is the bleed tick: two full rounds drain
/// exactly 2×`bleed_rate` (not 4×), proving the gate fires on the enemy phase alone.
#[test]
fn the_clock_ticks_once_per_full_round_not_per_turn_boundary() {
    let rate = bleed_rate();
    // Two rounds of headroom plus a cushion so neither round is lethal.
    let start = rate.saturating_mul(2).saturating_add(10);

    let mut app = live_app();
    let downed = bleeding_ganger(&mut app, PLAYER, LifeState::Downed, start);

    end_turn(&mut app); // round 1
    assert_eq!(
        wounds_of(&app, downed),
        start - rate,
        "after one full round the drop is exactly one bleed_rate (the enemy-phase start \
         ticks once — NOT once per TurnStarted, which would double it)",
    );

    end_turn(&mut app); // round 2
    assert_eq!(
        wounds_of(&app, downed),
        start - rate * 2,
        "two full rounds drain exactly 2×bleed_rate — once per round, at the enemy phase",
    );
}

/// GTW-336 terminal gate — the live clock kills. A Downed ganger with a Wounds pool that
/// empties in exactly two rounds is Dead after the second full round (Wounds floored at 0),
/// through the same once-only terminal gate as the fire path.
#[test]
fn the_live_clock_depletes_a_downed_ganger_to_dead() {
    let rate = bleed_rate();
    // Exactly two rounds' worth so the second round lands the pool on 0.
    let start = rate.saturating_mul(2);

    let mut app = live_app();
    let downed = bleeding_ganger(&mut app, PLAYER, LifeState::Downed, start);

    end_turn(&mut app);
    assert_eq!(
        life_of(&app, downed),
        LifeState::Downed,
        "the first round is non-lethal (Wounds still remain)",
    );

    end_turn(&mut app);
    assert_eq!(
        wounds_of(&app, downed),
        0,
        "bleeding to the floor leaves Wounds at 0 (saturating, no underflow)",
    );
    assert_eq!(
        life_of(&app, downed),
        LifeState::Dead,
        "the live clock emptying the pool transitions the ganger to Dead (the §9 gate)",
    );
}

/// GTW-336 filter — the live clock skips the gangers `tick_bleed` must skip. In one full
/// round: an Alive ganger and a Dead ganger are untouched and emit no `Bleeding`, and a
/// STABILIZED Downed ganger's clock is halted (no drain, no `Bleeding`) — only the
/// un-stabilized Downed ganger bleeds. This proves the whole `tick_bleed` filter runs over
/// the LIVE world, not just an isolated harness.
#[test]
fn the_live_clock_skips_alive_dead_and_stabilized_gangers() {
    let rate = bleed_rate();
    let start = rate.saturating_add(10);

    let mut app = live_app();
    let alive = bleeding_ganger(&mut app, PLAYER, LifeState::Alive, start);
    let dead = bleeding_ganger(&mut app, PLAYER, LifeState::Dead, start);
    let downed = bleeding_ganger(&mut app, ENEMY, LifeState::Downed, start);
    // A stabilized Downed ganger — an ally dressed the wound; the clock is halted.
    let stabilized = bleeding_ganger(&mut app, PLAYER, LifeState::Downed, start);
    app.world_mut()
        .entity_mut(stabilized)
        .insert(Stabilized::new(true));

    end_turn(&mut app);

    // The un-stabilized Downed ganger bled by exactly one rate; everyone else is untouched.
    assert_eq!(
        wounds_of(&app, downed),
        start - rate,
        "the Downed ganger bleeds"
    );
    assert_eq!(
        wounds_of(&app, alive),
        start,
        "an Alive ganger is up — never bleeds"
    );
    assert_eq!(
        wounds_of(&app, dead),
        start,
        "a Dead ganger is a corpse — never bleeds"
    );
    assert_eq!(
        wounds_of(&app, stabilized),
        start,
        "a stabilized Downed ganger's clock is halted — no drain (Wounds already lost stay \
         lost; it remains Downed)",
    );
    assert_eq!(
        life_of(&app, stabilized),
        LifeState::Downed,
        "the stabilized ganger remains Downed (the clock only halts; it does not revive)",
    );

    let bled = drain_bleeding(&mut app);
    assert_eq!(
        bled.len(),
        1,
        "exactly one Bleeding this round — only the un-stabilized Downed ganger: {bled:?}",
    );
    assert_eq!(
        bled.first().map(|b| b.ganger),
        Some(downed),
        "the single Bleeding carries the un-stabilized Downed ganger",
    );
}

/// GTW-336 inert outside a live battle — with NO `BattleInProgress` the Simulate band is
/// gated off, so even an `EndTurnRequested` neither cycles the turn nor bleeds a Downed
/// ganger. This pins the panic-free, no-op behavior the band gate guarantees
/// (`bevy-traps.md` #1) — the bleed wiring adds no unconditional work.
#[test]
fn no_battle_in_progress_means_no_bleed() {
    let rate = bleed_rate();
    let start = rate.saturating_add(10);

    // The live harness MINUS the BattleInProgress gate witness — the SAME BattleSimPlugin
    // registration (so the real Simulate-band gate IS configured), only the witness that
    // satisfies it is absent. The gate's run_if is false, so the whole band is skipped.
    let mut app = App::new();
    app.add_plugins(MinimalPlugins);
    app.add_plugins(BattleSimPlugin);
    seed_battle_resources(&mut app);

    let downed = bleeding_ganger(&mut app, PLAYER, LifeState::Downed, start);

    end_turn(&mut app);

    assert_eq!(
        wounds_of(&app, downed),
        start,
        "with no BattleInProgress the Simulate band is gated off — the Downed ganger does \
         not bleed (the wiring adds no unconditional work)",
    );
    assert!(
        drain_bleeding(&mut app).is_empty(),
        "no Bleeding is emitted outside a live battle",
    );
}
