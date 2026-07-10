//! GTW-695 — stabilize halts the bleed clock of a ganger downed on the REAL path.
//!
//! The regression the old negation-flag model silently failed: a ganger spawned through
//! the real combat-vitals path carried NO stabilization flag component, so the dispatcher's
//! `Some(&mut flag)` row never matched it and the stabilize act was skipped — the
//! ganger kept bleeding. These tests spawn a ganger with NO hand-inserted life component
//! (`LifeState::Alive` on spawn, the real vitals), down it FOR REAL — one through the
//! injury-HP bleed, one on the `apply_hit` DAMAGE path (the dominant weapon/melee/fall
//! route, reified by the live `mark_downed_bleeding` system, NOT a hand-insert) — stabilize
//! it through the REAL dispatcher, and assert the Wounds clock stops while the ganger stays
//! Downed. The second test is the dedicated pin on the `apply_hit` down-gate system.

use super::support::{BleedingOut, PLAYER, end_turn, life_of, live_app, wounds_of};
use crate::{
    acts::StabilizeDownedRequested,
    ganger::LifeState,
    injuries::BleedAfflicted,
    metric::{Cell, CellLevel, Level},
    test_support::GangerEntityBuilder,
};

/// A `(cell, level)` on the ground storey — terse position builder for the test bodies.
fn ground(x: i32, y: i32) -> CellLevel {
    CellLevel::new(Cell::new(x, y), Level::new(0))
}

/// GTW-695 — an ally stabilizes a REAL-spawned, REAL-downed ganger and its bleed clock
/// halts. Against the flag model this FAILS: the dispatcher skips the flag-less ganger,
/// so the Wounds keep draining after the stabilize.
#[test]
fn stabilize_halts_the_bleed_clock_of_a_real_spawned_downed_ganger() {
    let mut app = live_app();

    // The target — spawned through the real combat-vitals path (LifeState::Alive on
    // spawn, NOT a hand-inserted Downed), with a low Hp pool and an injury-bleed accrual
    // so tick_bleed's injury-HP bleed downs it FOR REAL over the first round.
    let target = GangerEntityBuilder::new()
        .at(ground(11, 10))
        .faction(PLAYER)
        .combat_vitals(1, 100)
        .tu(0)
        .tu_max(100)
        .spawn(app.world_mut());
    app.world_mut()
        .entity_mut(target)
        .insert(BleedAfflicted::new(5));

    // An adjacent Alive ally to run the stabilize.
    let ally = GangerEntityBuilder::new()
        .at(ground(10, 10))
        .faction(PLAYER)
        .combat_vitals(50, 10)
        .tu(0)
        .tu_max(100)
        .spawn(app.world_mut());

    // Round 1: the injury-HP bleed empties Hp (1 → 0) and downs the target for real; the
    // GTW-641 entry gate means no Wound drains the tick it falls.
    end_turn(&mut app);
    assert_eq!(
        life_of(&app, target),
        LifeState::Downed,
        "the injury-HP bleed downs the target for real (no hand-inserted Downed)",
    );

    // Round 2: it is now bleeding out — a Wound drains.
    let before = wounds_of(&app, target);
    end_turn(&mut app);
    let bleeding = wounds_of(&app, target);
    assert!(
        bleeding < before,
        "a real Downed ganger bleeds out (Wounds drop): {before} -> {bleeding}",
    );

    // The adjacent ally stabilizes it through the REAL dispatcher.
    app.world_mut()
        .write_message(StabilizeDownedRequested::new(ally, target));
    app.update();
    let stabilized_at = wounds_of(&app, target);

    // Two further rounds must NOT drain a Wound — the clock is halted.
    end_turn(&mut app);
    end_turn(&mut app);
    assert_eq!(
        wounds_of(&app, target),
        stabilized_at,
        "stabilize must halt the bleed clock of a REAL-spawned Downed ganger \
         (the flag model skipped it)",
    );
    assert_eq!(
        life_of(&app, target),
        LifeState::Downed,
        "a stabilized ganger stays Downed (the clock halts; it does not revive)",
    );
}

/// GTW-695 — the `apply_hit` down-gate discriminator. A ganger downed on the REAL damage
/// path (the `apply_hit` `LifeState::Downed` write, NOT the injury-HP bleed) must gain its
/// §9 `BleedingOut` condition from the LIVE `mark_downed_bleeding` system — never a
/// hand-insert — then bleed out and be stabilized through the real dispatcher. This is the
/// dominant down path (every weapon / melee / fall), and it has its OWN dedicated pin:
/// unwiring or emptying `mark_downed_bleeding` fails this test at the marker-present assert
/// (and again at the drain), because the marker never appears — precisely the
/// component-absence-silently-kills-the-act bug class this ticket reified away.
#[test]
fn a_damage_downed_ganger_gains_bleeding_out_from_the_live_system_and_stabilizes() {
    let mut app = live_app();

    // Spawned through the real combat-vitals path — Alive, NO hand-inserted Downed and NO
    // hand-inserted marker.
    let target = GangerEntityBuilder::new()
        .at(ground(11, 10))
        .faction(PLAYER)
        .combat_vitals(50, 10)
        .tu(0)
        .tu_max(100)
        .spawn(app.world_mut());
    let ally = GangerEntityBuilder::new()
        .at(ground(10, 10))
        .faction(PLAYER)
        .combat_vitals(50, 10)
        .tu(0)
        .tu_max(100)
        .spawn(app.world_mut());

    // Down it on the DAMAGE path: write LifeState::Downed directly — the exact transition
    // `apply_hit`'s terminal gate applies in place (the pure fold carries no Commands, so
    // the live runtime never marks at the write; `mark_downed_bleeding` reacts to it).
    app.world_mut().entity_mut(target).insert(LifeState::Downed);
    assert!(
        app.world().get::<BleedingOut>(target).is_none(),
        "no marker until the live system reacts (nothing hand-inserted it)",
    );

    // One sim tick, no turn boundary — only `mark_downed_bleeding` runs (tick_bleed is
    // gated on the enemy-phase boundary). It must reify the down as the condition.
    app.update();
    assert!(
        app.world().get::<BleedingOut>(target).is_some(),
        "the live mark_downed_bleeding must reify the apply_hit down as BleedingOut — \
         unwiring/emptying it drops this (and the whole weapon/melee/fall bleed-out path)",
    );

    // The drain follows end-to-end: two rounds bleed Wounds off the now-conditioned ganger.
    let before = wounds_of(&app, target);
    end_turn(&mut app);
    end_turn(&mut app);
    let bleeding = wounds_of(&app, target);
    assert!(
        bleeding < before,
        "a damage-downed ganger bleeds out via the live-reified condition: {before} -> {bleeding}",
    );

    // Stabilize through the REAL dispatcher removes the condition and halts the clock.
    app.world_mut()
        .write_message(StabilizeDownedRequested::new(ally, target));
    app.update();
    assert!(
        app.world().get::<BleedingOut>(target).is_none(),
        "stabilize removes the BleedingOut condition",
    );
    let stabilized_at = wounds_of(&app, target);
    end_turn(&mut app);
    end_turn(&mut app);
    assert_eq!(
        wounds_of(&app, target),
        stabilized_at,
        "stabilize halts the clock of a damage-downed ganger",
    );
    assert_eq!(
        life_of(&app, target),
        LifeState::Downed,
        "a stabilized ganger stays Downed (the clock halts; it does not revive)",
    );
}
