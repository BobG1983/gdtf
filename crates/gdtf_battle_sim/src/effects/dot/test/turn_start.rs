//! GTW-658 — the boundary-frame DETERMINISM pin for the GTW-544 DOT clock (the
//! GTW-641 `tick_bleed` precedent, the `effects/bleed/test/turn_start.rs` shape): on
//! the boundary frame, with a same-frame enemy act in flight, the clock's effect
//! resolves BEFORE the act's — `tick_dot` is pinned `.before` the act dispatchers in
//! `acts/plugin/turn_clocks.rs` (clocks resolve AT the boundary, before the new
//! turn's act resolution). Without the pin the interleaving is a scheduler lottery
//! (bevy-traps.md #3); this test makes a lost lottery a red, not a flake.

use bevy::prelude::{App, Messages, MinimalPlugins};

use super::support::{dot, ground, hp_of, life_of};
use crate::{
    acts::{EndTurnRequested, MoveRequested, MovementOccurred},
    battle::{BattleInProgress, BattleRoster, BattleSimPlugin},
    effects::dot::DotTicked,
    ganger::{Faction, LifeState, Position},
    rng::BattleSeed,
    test_support::{GangerEntityBuilder, full_vision, insert_sim_resources},
    turn::ActiveFaction,
};

/// A fixed seed for the per-test RNG streams (arbitrary, not tuned).
const SEED: u64 = 0x5A1C_AC75;

/// Gang `0` is the player team (the litany's `PlayerFaction` gang).
const PLAYER: Faction = Faction::new(0);
/// The enemy team — the boundary frame's incoming (acting) faction.
const ENEMY: Faction = Faction::new(1);

/// Build the FULL live-runtime harness (the bleed turn-start pin's composition):
/// [`MinimalPlugins`] + the real production [`BattleSimPlugin`] (the turn-cycle
/// engine, the dispatchers, and the GTW-658-pinned clock registration), seeded with
/// the canonical sim-resource litany, the [`BattleRoster`] census read, the player
/// holding [`ActiveFaction`], and the [`BattleInProgress`] gate witness — so this
/// test exercises the SAME registration the live runtime uses, not a hand-rolled
/// subset.
fn live_app() -> App {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins);
    app.add_plugins(BattleSimPlugin);
    insert_sim_resources(&mut app, BattleSeed::new(SEED));
    app.insert_resource(BattleRoster::new([PLAYER, ENEMY]));
    app.insert_resource(ActiveFaction::new(PLAYER));
    app.insert_resource(BattleInProgress);
    app
}

/// Send one End-Turn request and run ONE update — the frame the enemy-phase
/// `TurnStarted` boundary lands on (the frame under test).
fn end_turn(app: &mut App) {
    app.world_mut().write_message(EndTurnRequested);
    app.update();
}

/// Drain the buffered [`DotTicked`] signals emitted so far, in order.
fn drain_dot_ticks(app: &mut App) -> Vec<DotTicked> {
    app.world_mut()
        .resource_mut::<Messages<DotTicked>>()
        .drain()
        .collect()
}

/// Drain the buffered [`MovementOccurred`] signals emitted so far — the proof the
/// same-frame act actually RESOLVED (a rejected move would make the pin vacuous).
fn drain_movements(app: &mut App) -> Vec<MovementOccurred> {
    app.world_mut()
        .resource_mut::<Messages<MovementOccurred>>()
        .drain()
        .collect()
}

/// GTW-658 — the DOT clock's effect resolves BEFORE a same-frame enemy act's: on the
/// boundary frame an afflicted enemy mover has a real one-cell [`MoveRequested`] in
/// flight, and the [`DotTicked`] the boundary tick emits carries the mover's
/// TURN-START cell (the clock read the world BEFORE the act's step), while the act
/// still resolves the same frame (one [`MovementOccurred`], the mover ends at the
/// destination). An unpinned schedule that ran the clock after the walk would carry
/// the post-step cell — the nondeterministic interleaving this pin outlaws.
#[test]
fn the_boundary_dot_tick_resolves_before_a_same_frame_enemy_act() {
    let per_turn = 5u16;
    let start_hp = per_turn + 10; // non-lethal — the pin is about ORDER, not the kill
    let origin = ground(10, 10);
    let dest = ground(11, 10);

    let mut app = live_app();
    // The route gate reads the squad fog — full vision so the move resolves on
    // geometry + occupancy alone (the acts-suite precondition).
    app.insert_resource(full_vision());

    // The afflicted ENEMY mover — the boundary frame's actor. Spawned WITHOUT the
    // AI's required row (no Stance/Facing/Aiming), so the enemy brain plans nothing
    // for it and the hand-written request below is the ONLY enemy act in flight.
    let mover = GangerEntityBuilder::new()
        .at(origin)
        .faction(ENEMY)
        .life_state(LifeState::Alive)
        .hp(start_hp)
        .tu(100)
        .tu_max(100)
        .spawn(app.world_mut());
    app.world_mut().entity_mut(mover).insert(dot(per_turn, 3));

    // Mid-turn frames settle occupancy (the maintenance chain projects the spawn) —
    // and the clock must NOT tick without a turn boundary.
    app.update();
    app.update();
    assert_eq!(
        hp_of(&app, mover),
        start_hp,
        "plain mid-turn frames must not tick the DOT clock (no turn boundary crossed)",
    );
    assert!(
        drain_dot_ticks(&mut app).is_empty(),
        "no DotTicked may be emitted mid-turn",
    );

    // The same-frame enemy act in flight: a one-cell move, dispatched on the SAME
    // frame the enemy TurnStarted lands (the dispatchers run after the handoff).
    app.world_mut()
        .write_message(MoveRequested::new(mover, dest));
    end_turn(&mut app);

    // THE PIN — the clock's effect resolved BEFORE the act's: the boundary tick's
    // DotTicked carries the mover's turn-start cell, not its post-step one.
    let ticks = drain_dot_ticks(&mut app);
    assert_eq!(
        ticks.len(),
        1,
        "the boundary frame ticks the DOT clock exactly once: {ticks:?}",
    );
    assert!(
        ticks.iter().all(|tick| tick.at == origin),
        "the boundary DotTicked lands at the mover's TURN-START cell — the clock \
         resolves BEFORE the same-frame act's step (GTW-658): {ticks:?}",
    );
    assert_eq!(
        hp_of(&app, mover),
        start_hp - per_turn,
        "the boundary tick drains exactly one per-turn amount",
    );
    assert_eq!(
        life_of(&app, mover),
        LifeState::Alive,
        "the non-lethal boundary tick leaves the mover Alive",
    );

    // Non-vacuousness: the same-frame act actually RESOLVED — one real step, the
    // mover ends at the destination (the clock ordered before it, never blocked it).
    let movements = drain_movements(&mut app);
    assert_eq!(
        movements.len(),
        1,
        "the same-frame enemy move must actually RESOLVE (one MovementOccurred) — \
         otherwise this pin proves nothing: {movements:?}",
    );
    let at = app.world().get::<Position>(mover).map(|p| **p);
    assert_eq!(
        at,
        Some(dest),
        "the mover ends the boundary frame at the destination (the act resolved \
         AFTER the clock)",
    );
}
