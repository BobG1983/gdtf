//! GTW-366 — the ground-accrual path through the REAL fold
//! ([`resolve_and_apply`](super::super::resolve_and_apply)): a `ShotKind::Ground`
//! outcome records the round's `weapon_damage` against the struck ground-plane
//! [`Cell`] in [`HitReport::ground_accrued`], mutating NOTHING — the ground is
//! damaged, never destroyed. The cosmetic mirror of [`super::cover`] / [`super::slab`].
//!
//! These assert the MECHANISM, never a shipped magnitude (C7): the accrual relation
//! (`report.ground_accrued.amount == the round's weapon_damage`, on the CORRECT cell),
//! that NO other state is touched (no applied damage, no cover / slab depletion), and
//! that the path is replay-deterministic (it draws NO RNG). The monotonic-sum property
//! is driven end-to-end through the production `dispatch_fire` + `sync_accrued_ground`
//! wiring in `tests/gtw366_ground_accrued_bridge.rs`.

use super::support::*;

/// C7(a) — a `ShotKind::Ground` hit records the round's `weapon_damage` on the report's
/// `ground_accrued`, keyed to the CORRECT ground-plane cell.
///
/// The accrued amount is the WEAPON's damage (C4 — not a constant, not a tuning leaf):
/// the test reads it back relative to the weapon it built, never a pinned number.
#[test]
fn ground_hit_accrues_weapon_damage_on_the_correct_cell() {
    let tuning = CombatTuning::default();
    // An arbitrary (NOT shipped-tuning) per-round damage — the test asserts the accrual
    // equals THIS value, never a hardcoded magnitude.
    let weapon_damage = 37_i32;
    let weapon = a_weapon(weapon_damage, 10, 4, DamageType::Kinetic);
    let at = ground_cell_level();
    let expected_cell = at.cell();

    let mut cover = ledger();
    let mut slab = slab_ledger();

    let mut rng_used = rng();
    let report = resolve_and_apply(
        &ground_outcome(),
        weapon.stats(),
        Luck::new(0.0),
        // No struck ganger on a ground hit.
        None,
        an_entity(),
        surfaces(&mut cover, &mut slab),
        &tuning,
        &mut rng_used,
        &injury_tables(),
        &injury_registry(),
        &mut injury_rng(),
    );

    // The fold must record a ground-accrual verdict — C4: the accrued amount IS the
    // round's weapon_damage, keyed to the ground-plane cell (read relative to the
    // weapon the test built — the inner u32 of the damage value, never a shipped
    // magnitude). Any other verdict variant means the Ground arm did not accrue.
    assert_eq!(
        report.verdict,
        HitVerdict::Ground(GroundAccrual::new(
            expected_cell,
            GroundDamage::new(u32::try_from(weapon_damage).unwrap_or(0)),
        )),
        "a ground hit must record the round's weapon_damage against the struck cell",
    );
}

/// C7(c) — a ground hit mutates ONLY the report's accrual: it wounds no ganger, depletes
/// no cover, depletes no slab, and records no destruction. (The accumulator mutation
/// itself is a SEPARATE resource the fold never holds — the report is the fold's sole
/// output; the grid edit happens in `sync_accrued_ground`, asserted in the bridge test.)
#[test]
fn ground_hit_touches_no_ganger_cover_or_slab_state() {
    let tuning = CombatTuning::default();
    let weapon = a_weapon(50, 30, 12, DamageType::Kinetic);

    // Seed BOTH structural ledgers with a known entry at the ground cell, so a stray
    // depletion would be visible. A ground hit must leave both EXACTLY as seeded.
    let at = ground_cell_level();
    let mut cover = ledger();
    cover.insert(at, cover_entry(100, 2, 1));
    let cover_before = cover.peek(&at).copied();
    let mut slab = slab_ledger();
    slab.insert(at, slab_entry(100, 2, 1));
    let slab_before = slab.peek(&at).copied();

    let mut rng_used = rng();
    let report = resolve_and_apply(
        &ground_outcome(),
        weapon.stats(),
        Luck::new(0.0),
        None,
        an_entity(),
        surfaces(&mut cover, &mut slab),
        &tuning,
        &mut rng_used,
        &injury_tables(),
        &injury_registry(),
        &mut injury_rng(),
    );

    // The verdict carries ONLY the accrual — an applied ganger wound / a cover / slab
    // destruction is structurally impossible on a Ground verdict (GTW-573).
    assert!(
        matches!(report.verdict, HitVerdict::Ground(_)),
        "a ground hit must fold to a Ground accrual verdict, got {:?}",
        report.verdict,
    );

    // Neither structural ledger was touched — both read EXACTLY as seeded (the fold
    // never spent their HP). The cover / slab entries are `Copy`, so an unchanged
    // snapshot is the witness.
    assert_eq!(
        cover.peek(&at).copied(),
        cover_before,
        "a ground hit must not deplete the cover ledger (C3 — only the ground accrues)",
    );
    assert_eq!(
        slab.peek(&at).copied(),
        slab_before,
        "a ground hit must not deplete the slab ledger (C3 — only the ground accrues)",
    );
}

/// C7 — the ground path is replay-DETERMINISTIC: it draws NO RNG (the same seed is
/// untouched), and the same inputs reproduce the same accrual verdict.
#[test]
fn ground_hit_takes_no_rng_draw_and_is_deterministic() {
    let run = || {
        let tuning = CombatTuning::default();
        let weapon = a_weapon(29, 14, 6, DamageType::Kinetic);
        let mut cover = ledger();
        let mut slab = slab_ledger();
        let mut r = rng();
        let report = resolve_and_apply(
            &ground_outcome(),
            weapon.stats(),
            Luck::new(0.0),
            None,
            an_entity(),
            surfaces(&mut cover, &mut slab),
            &tuning,
            &mut r,
            &injury_tables(),
            &injury_registry(),
            &mut injury_rng(),
        );
        report.verdict
    };

    // Same seed + inputs reproduce the same accrual.
    assert_eq!(
        run(),
        run(),
        "the same seed + inputs must reproduce the same ground accrual",
    );

    // The ground path consumed NO draw: a fresh stream's first draw matches the
    // post-fold stream's first draw (the cursor never advanced).
    let tuning = CombatTuning::default();
    let weapon = a_weapon(29, 14, 6, DamageType::Kinetic);
    let mut cover = ledger();
    let mut slab = slab_ledger();
    let mut rng_used = rng();
    // Bind the report (HitReport has a destructor since GTW-438 — it owns the rolled
    // injury — so `let _ =` is flagged); the test asserts the RNG state, not the report.
    let _report = resolve_and_apply(
        &ground_outcome(),
        weapon.stats(),
        Luck::new(0.0),
        None,
        an_entity(),
        surfaces(&mut cover, &mut slab),
        &tuning,
        &mut rng_used,
        &injury_tables(),
        &injury_registry(),
        &mut injury_rng(),
    );
    let mut rng_fresh = rng();
    assert_eq!(
        rng_used.next_u64(),
        rng_fresh.next_u64(),
        "a ground hit must take NO RNG draw (replay-deterministic)",
    );
}
