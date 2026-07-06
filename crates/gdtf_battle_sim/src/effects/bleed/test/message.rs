//! AC6 — the buffered `Bleeding` message: one per draining Downed ganger carrying
//! the right `Entity`, plus the lethal-tick emit+gate-on-the-same-tick and the
//! once-only skip of an already-dead (bled-out) ganger.

use super::support::{
    Bleeding, LifeState, Stabilized, Wounds, bleed_app, bleed_rate, bleeding_count_for, life_of,
    wounds_of,
};
use crate::ganger::Hp;

/// AC6 — `Bleeding` is a buffered `#[derive(Message)]` (NOT the observer `Event`
/// API, `bevy-traps.md` #4), read with a `MessageReader`: one per draining Downed
/// ganger carrying the right `Entity`, and none for a stabilized or alive ganger.
/// Spawns a Downed, a Stabilized-Downed, and an Alive ganger, ticks once, and
/// asserts exactly one captured `Bleeding` — for the bleeding ganger only.
#[test]
fn bleeding_is_a_buffered_message_one_per_draining_ganger() {
    let rate = bleed_rate();
    let start = rate.saturating_add(10);

    let mut app = bleed_app();
    let bleeder = app
        .world_mut()
        .spawn((Wounds::new(start), LifeState::Downed))
        .id();
    let stabilized = app
        .world_mut()
        .spawn((Wounds::new(start), LifeState::Downed, Stabilized::new(true)))
        .id();
    let alive = app
        .world_mut()
        .spawn((Wounds::new(start), LifeState::Alive))
        .id();

    app.update();

    let captured = app
        .world()
        .get_resource::<super::support::Captured>()
        .map_or_else(Vec::new, |c| c.to_vec());

    // Exactly one Bleeding overall — only the un-stabilized Downed ganger bled.
    assert_eq!(
        captured.len(),
        1,
        "exactly one Bleeding must be buffered (one per draining Downed ganger)",
    );
    assert_eq!(
        captured.first(),
        Some(&Bleeding::new(bleeder)),
        "the buffered Bleeding must carry the bleeding ganger's Entity",
    );
    // None for the stabilized or alive ganger.
    assert_eq!(
        bleeding_count_for(&app, stabilized),
        0,
        "a stabilized Downed ganger emits no Bleeding",
    );
    assert_eq!(
        bleeding_count_for(&app, alive),
        0,
        "an Alive ganger emits no Bleeding",
    );
}

/// The lethal tick still emits a `Bleeding`: the draining-to-empty tick both
/// emits the per-tick signal AND flips the ganger to Dead (drain + emit + gate on
/// the same tick, including the lethal one). Spawns a ganger with exactly one
/// round of bleed left and asserts both happen on that single tick.
#[test]
fn the_lethal_tick_emits_bleeding_and_flips_to_dead() {
    let rate = bleed_rate();
    // Exactly one round of bleed left, so this single tick is the lethal one.
    let start = rate;

    let mut app = bleed_app();
    let ganger = app
        .world_mut()
        .spawn((Wounds::new(start), LifeState::Downed))
        .id();

    app.update();

    assert_eq!(
        wounds_of(&app, ganger),
        0,
        "the lethal tick must drain the pool to 0",
    );
    assert_eq!(
        life_of(&app, ganger),
        LifeState::Dead,
        "the lethal tick must flip the ganger to Dead (the terminal gate)",
    );
    assert_eq!(
        bleeding_count_for(&app, ganger),
        1,
        "the lethal tick still emits one Bleeding (drain + emit + gate on the same tick)",
    );
}

/// The once-only property end to end: a ganger bled to death is skipped by the
/// NEXT tick — no second drain, no second `Bleeding`. After the lethal tick (Dead,
/// Wounds 0), a further `update()` must mutate nothing and emit nothing.
#[test]
fn a_dead_bled_out_ganger_is_skipped_next_tick() {
    let rate = bleed_rate();
    let start = rate; // one round to death

    let mut app = bleed_app();
    let ganger = app
        .world_mut()
        .spawn((Wounds::new(start), LifeState::Downed, Hp::new(0)))
        .id();

    // Tick 1: the lethal tick — Dead, Wounds 0, one Bleeding.
    app.update();
    assert_eq!(life_of(&app, ganger), LifeState::Dead);
    assert_eq!(bleeding_count_for(&app, ganger), 1);

    // Tick 2: the corpse is skipped — still one Bleeding total, still Dead.
    app.update();
    assert_eq!(
        life_of(&app, ganger),
        LifeState::Dead,
        "a dead ganger stays Dead across the next tick",
    );
    assert_eq!(
        bleeding_count_for(&app, ganger),
        1,
        "a dead (bled-out) ganger emits NO second Bleeding (the once-only property)",
    );
}
