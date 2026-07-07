//! The per-round [`tick_dot`](crate::effects::dot::tick_dot) drain pins: the flat
//! HP-decrement, the GTW-643 decrement-or-REMOVE expiry (no zero-valued `Dot` ever
//! exists between ticks), the DOT-kills terminal gate, and the pinned already-Dead
//! skip contract.

use super::support::{
    Hp, LifeState, Position, dot, dot_of, ground, hp_of, life_of, tick_app, tick_count_for,
};

// === (c) tick_dot decrements HP each round with NO armor/injury/RNG, removes after N turns. ===

#[test]
fn tick_dot_drains_hp_each_round_and_removes_after_the_profile_turn_count() {
    // A 3-turn DOT of 5 HP/turn on a deep HP pool (so no tick reaches 0 — we measure the
    // per-turn drop + the removal, not the kill).
    let per_turn = 5u16;
    let turns = 3u8;
    let start_hp = 100u16;

    let mut app = tick_app();
    let ganger = app
        .world_mut()
        .spawn((
            Hp::new(start_hp),
            LifeState::Alive,
            dot(per_turn, turns),
            Position::new(ground(4, 4)),
        ))
        .id();

    // Each of the `turns` ticks drains exactly `per_turn` HP (no armor / injury / RNG — a
    // flat direct drain) and the ganger stays Alive.
    for n in 1..=turns {
        app.update();
        assert_eq!(
            hp_of(&app, ganger),
            start_hp - per_turn * u16::from(n),
            "after {n} ticks the cumulative HP drop must be {n}×per_turn (a flat direct drain)",
        );
        assert_eq!(
            life_of(&app, ganger),
            LifeState::Alive,
            "a non-lethal DOT tick leaves the ganger Alive",
        );
        // GTW-643 (A3): between ticks the affliction is either PRESENT with a positive
        // remaining count (turns − n, decremented in place) or GONE (the expiring tick
        // removed it) — a zero-valued Dot never exists between ticks.
        assert_eq!(
            dot_of(&app, ganger).map(|d| d.remaining_turns.get()),
            (n < turns).then_some(turns - n),
            "after {n} of {turns} ticks the DOT is decremented in place, then REMOVED on \
             the expiring tick — never stored at zero",
        );
    }

    // After the profile turn count the DOT is REMOVED — a further tick drains nothing.
    assert!(
        dot_of(&app, ganger).is_none(),
        "the DOT is removed once its profile turn count runs out",
    );
    let hp_after_expiry = hp_of(&app, ganger);
    app.update();
    assert_eq!(
        hp_of(&app, ganger),
        hp_after_expiry,
        "with the DOT removed a further round drains no HP",
    );
    // Exactly `turns` ticks were emitted (none after removal).
    assert_eq!(
        tick_count_for(&app, ganger),
        usize::from(turns),
        "exactly one DotTicked per turn of the profile (none after removal)",
    );
}

// === GTW-643 (A3): expiry = decrement-or-REMOVE — the last tick removes the component. ===

/// The minimal one-turn DOT: its single tick drains once and REMOVES the component in the
/// same round — the affliction is GONE at expiry, never left inert (pre-GTW-643 the expiring
/// tick stored a zero-turn `Dot`; with zero unrepresentable, removal IS the expiry. This is
/// the green successor of the C1 red leak repro, whose 0-turn construction no longer
/// compiles — see the `compile_fail` pin on [`crate::weapon::DotTurns`]).
#[test]
fn the_expiring_tick_removes_the_dot_not_an_inert_component() {
    let mut app = tick_app();
    let ganger = app
        .world_mut()
        .spawn((
            Hp::new(50),
            LifeState::Alive,
            dot(5, 1),
            Position::new(ground(1, 1)),
        ))
        .id();

    app.update();

    // The single turn drained once …
    assert_eq!(
        hp_of(&app, ganger),
        45,
        "the one-turn DOT drains exactly once before expiring",
    );
    assert_eq!(
        tick_count_for(&app, ganger),
        1,
        "exactly one DotTicked for the one-turn DOT",
    );
    // … and the component is GONE at expiry — removed, not stored at zero / left inert.
    assert!(
        dot_of(&app, ganger).is_none(),
        "the expiring tick must REMOVE the Dot component (decrement-or-REMOVE, GTW-643)",
    );
}

// === (d) a DOT tick that brings HP to 0 flips LifeState to Dead. ===

#[test]
fn a_dot_tick_that_empties_hp_flips_the_ganger_to_dead() {
    // A single 10 HP/turn tick on a ganger with exactly 8 HP — the tick floors HP at 0
    // (saturating) and the GTW-544 locked gate flips it to Dead (NOT Downed).
    let mut app = tick_app();
    let ganger = app
        .world_mut()
        .spawn((
            Hp::new(8),
            LifeState::Alive,
            dot(10, 3),
            Position::new(ground(2, 2)),
        ))
        .id();

    app.update();

    assert_eq!(
        hp_of(&app, ganger),
        0,
        "the DOT tick floors HP at 0 (saturating, no underflow)",
    );
    assert_eq!(
        life_of(&app, ganger),
        LifeState::Dead,
        "the GTW-544 locked design: a DOT tick that empties HP KILLS (Dead, not Downed)",
    );
    // The DOT is removed on the killing tick (a corpse never ticks again).
    assert!(
        dot_of(&app, ganger).is_none(),
        "the DOT is removed on the killing tick",
    );
}

// === GTW-643 (C3): the already-Dead host contract, pinned — read, not changed. ===

/// A ganger already [`LifeState::Dead`] at tick start is skipped entirely: no drain, no
/// [`DotTicked`](crate::effects::dot::DotTicked) emission, and its `Dot` REMAINS on the
/// corpse untouched (turns not decremented). This is the pre-GTW-643 Dead-path contract
/// (`tick_dot` step 1 — only a KILLING tick removes the corpse's Dot), pinned here so the
/// GTW-643 tick reshape provably did not change it silently.
#[test]
fn an_already_dead_hosts_dot_is_skipped_untouched() {
    let start_hp = 30u16;
    let turns = 2u8;

    let mut app = tick_app();
    let corpse = app
        .world_mut()
        .spawn((
            Hp::new(start_hp),
            LifeState::Dead,
            dot(5, turns),
            Position::new(ground(3, 3)),
        ))
        .id();

    app.update();

    assert_eq!(
        hp_of(&app, corpse),
        start_hp,
        "a corpse's DOT drains nothing (the once-only property)",
    );
    assert_eq!(
        tick_count_for(&app, corpse),
        0,
        "a corpse's DOT emits no DotTicked",
    );
    assert_eq!(
        dot_of(&app, corpse).map(|d| d.remaining_turns.get()),
        Some(turns),
        "the corpse's Dot remains, inert and undecremented (the pinned Dead-path contract)",
    );
}
