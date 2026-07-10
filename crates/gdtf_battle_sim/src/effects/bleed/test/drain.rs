//! AC1/AC2 — the per-tick drain magnitude and the stacking clock that depletes a
//! Downed ganger to Dead.

use super::support::{BleedingOut, LifeState, Wounds, bleed_app, bleed_rate, life_of, wounds_of};

/// AC1 — one `tick_bleed` drains exactly `bleed_rate` Wounds from an un-stabilized
/// Downed ganger. Spawns a Downed ganger with a comfortable Wounds pool, runs one
/// `update()`, and asserts the drop equals the tuning rate (a RELATION to the
/// tuning value, never a pinned magnitude).
#[test]
fn one_tick_drains_exactly_bleed_rate_from_a_downed_ganger() {
    let rate = bleed_rate();
    // A pool well above the rate so this single tick can't reach the gate (we are
    // measuring the per-tick drop, not the death transition).
    let start = rate.saturating_add(10);

    let mut app = bleed_app();
    let ganger = app
        .world_mut()
        .spawn((Wounds::new(start), LifeState::Downed, BleedingOut))
        .id();

    app.update();

    assert_eq!(
        wounds_of(&app, ganger),
        start - rate,
        "one tick must drain exactly the tuning bleed_rate from a Downed ganger",
    );
    assert_eq!(
        life_of(&app, ganger),
        LifeState::Downed,
        "a non-lethal tick must leave the ganger Downed",
    );
}

/// AC2 — the clock is a stack: N ticks drain N×`bleed_rate`, and ticking until the
/// pool empties flips the ganger to `Dead` via the terminal gate. Drives several
/// ticks asserting the cumulative drop, then keeps ticking to depletion and
/// asserts the Dead transition (Wounds floored at 0).
#[test]
fn ticks_stack_and_deplete_to_dead() {
    let rate = bleed_rate();
    // A multiple of the rate so depletion lands cleanly at 0 (and is > 0 so the
    // first tick is non-lethal). Five rounds of bleed.
    let rounds = 5u8;
    let start = rate.saturating_mul(rounds);

    let mut app = bleed_app();
    let ganger = app
        .world_mut()
        .spawn((Wounds::new(start), LifeState::Downed, BleedingOut))
        .id();

    // Tick three rounds — cumulative drop is 3×rate, still alive-but-downed.
    for n in 1..=3u8 {
        app.update();
        assert_eq!(
            wounds_of(&app, ganger),
            start - rate * n,
            "after {n} ticks the cumulative drop must be {n}×bleed_rate (a stack)",
        );
        assert_eq!(
            life_of(&app, ganger),
            LifeState::Downed,
            "while Wounds remain the ganger stays Downed",
        );
    }

    // Keep ticking until the pool empties — the terminal gate flips it to Dead.
    for _ in 0..rounds {
        app.update();
    }

    assert_eq!(
        wounds_of(&app, ganger),
        0,
        "bleeding to the floor must leave Wounds at 0 (saturating, no underflow)",
    );
    assert_eq!(
        life_of(&app, ganger),
        LifeState::Dead,
        "Wounds depleted to 0 by the clock must transition the ganger to Dead",
    );
}
