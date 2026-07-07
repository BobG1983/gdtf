//! GTW-641 (C3) — the TURN-START timing pin: the §9 bleed-out clock ticks at a turn
//! START (the once-per-full-round enemy-phase `TurnStarted` boundary,
//! `docs/combat/resolution.md` §9) and NEVER mid-turn — neither from plain frames
//! passing nor from an act resolving through its dispatcher. The user ruling on the
//! ticket ("Bleeds should happen at turn start") is already the live wiring
//! (`acts/plugin/turn_clocks.rs` gates `tick_bleed` on
//! [`enemy_phase_started`](crate::effects::bleed::enemy_phase_started)); this test
//! pins it so a future rewire that ticks bleeds from act resolution goes red.

use bevy::prelude::Messages;

use super::support::{PLAYER, bleed_rate, bleeding_ganger, end_turn, life_of, live_app, wounds_of};
use crate::{
    acts::{MoveRequested, MovementOccurred},
    ganger::LifeState,
    metric::{Cell, CellLevel, Level},
    test_support::{GangerEntityBuilder, full_vision},
};

/// Drain the buffered [`MovementOccurred`] signals emitted so far — the proof the
/// mid-turn act actually RESOLVED (a rejected move would make the pin vacuous).
fn drain_movements(app: &mut bevy::prelude::App) -> Vec<MovementOccurred> {
    app.world_mut()
        .resource_mut::<Messages<MovementOccurred>>()
        .drain()
        .collect()
}

/// GTW-641 (C3) — a bleed must not tick MID-TURN: with the player turn active, plain
/// frames pass and a REAL move act resolves through `dispatch_move`, and the Downed
/// ganger's Wounds stay untouched; the drain lands exactly ONCE, at the next
/// turn-start boundary (the enemy-phase `TurnStarted` the End Turn crosses).
#[test]
fn a_mid_turn_act_does_not_tick_the_bleed_clock() {
    let rate = bleed_rate();
    let start = rate.saturating_add(10);

    let mut app = live_app();
    // The route gate reads the squad fog — full vision so the move resolves on
    // geometry + occupancy alone (the acts-suite precondition).
    app.insert_resource(full_vision());

    let downed = bleeding_ganger(&mut app, PLAYER, LifeState::Downed, start);
    // An Alive player mover — the mid-turn actor whose act resolution must NOT
    // tick the clock.
    let mover = GangerEntityBuilder::new()
        .at(CellLevel::new(Cell::new(10, 10), Level::new(0)))
        .faction(PLAYER)
        .life_state(LifeState::Alive)
        .tu(100)
        .spawn(app.world_mut());

    // Mid-turn frames pass (no turn boundary) — the clock must not tick per-frame.
    app.update();
    app.update();
    assert_eq!(
        wounds_of(&app, downed),
        start,
        "plain mid-turn frames must not tick the bleed clock (no turn boundary crossed)",
    );

    // A REAL act resolves mid-turn: a one-cell move through dispatch_move.
    let dest = CellLevel::new(Cell::new(11, 10), Level::new(0));
    app.world_mut()
        .write_message(MoveRequested::new(mover, dest));
    app.update();
    let movements = drain_movements(&mut app);
    assert_eq!(
        movements.len(),
        1,
        "the mid-turn move must actually RESOLVE (one MovementOccurred) — otherwise \
         this pin proves nothing: {movements:?}",
    );
    assert_eq!(
        wounds_of(&app, downed),
        start,
        "an act resolving mid-turn must NOT tick the bleed clock (GTW-641: bleeds \
         happen at turn start, never from act resolution)",
    );

    // The drain lands exactly at the turn-start boundary — one End Turn, one rate.
    end_turn(&mut app);
    assert_eq!(
        wounds_of(&app, downed),
        start - rate,
        "the bleed drains exactly once, at the turn-start boundary the End Turn crossed",
    );
    assert_eq!(
        life_of(&app, downed),
        LifeState::Downed,
        "the non-lethal boundary tick leaves the ganger Downed",
    );
}
