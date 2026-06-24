//! GTW-364 — the cover-hit path through the REAL fold
//! ([`resolve_and_apply`](super::super::resolve_and_apply)): a `ShotKind::Cover`
//! outcome reuses the ganger damage formula against the cover's own armor, spends the
//! [`CoverLedger`]'s HP via `deplete_cover`, and records a destroyed `(cell, level)` on
//! the report ONLY when the hit depletes the HP to zero.
//!
//! These assert the MECHANISM, not magnitudes (C7 / C8): the relations "HP fell to
//! destruction + a destroyed cell was recorded" (a SUFFICIENT hit) and "HP fell but
//! the piece still stands + no cell recorded" (an INSUFFICIENT hit) — never a pinned
//! HP number. Every fixture armor/weapon magnitude is chosen only to land the hit in
//! the regime under test, never as a balance claim.

use super::support::*;

/// C8(a) — a SUFFICIENT hit depletes cover HP to destruction AND records the
/// destroyed `(cell, level)` on the report.
///
/// The weapon's damage far exceeds the cover's (low) protection + (small) HP, so the
/// reused `resolve_hit` damage spends the ledger's HP to zero: `deplete_cover` returns
/// `Destroyed`, the ledger's entry reads destroyed at zero HP, and the report carries
/// `cover_destroyed == Some(the struck cell)`. Magnitudes are NOT pinned — only the
/// destruction relation is.
#[test]
fn sufficient_hit_destroys_cover_and_records_the_cell() {
    let tuning = CombatTuning::default();
    // High damage, low cover armor + small HP → the hit breaches and empties the pool.
    let weapon = a_weapon(60, 40, 20, DamageType::Kinetic);
    let entry = cover_entry(20, 2, 1);
    let at = cover_cell_level();

    let mut cover = ledger();
    // Seed the ledger's entry at the struck cell so we can read its HP after the hit.
    cover.insert(at, entry);

    let mut rng_used = rng();
    let report = resolve_and_apply(
        &cover_outcome(entry),
        weapon.stats(),
        Luck::new(0.0),
        // No struck ganger on a cover hit.
        None,
        an_entity(),
        surfaces(&mut cover, &mut slab_ledger()),
        &tuning,
        &mut rng_used,
    );

    // The report records the destroyed cell (the bridge dispatch_fire turns into a
    // CoverDestroyed message).
    assert_eq!(
        report.cover_destroyed,
        Some(at),
        "a sufficient cover hit must record the destroyed (cell, level) on the report",
    );
    assert_eq!(report.applied, None, "a cover hit wounds no ganger");
    assert_eq!(report.part, None, "a cover hit has no struck body part");

    // The ledger spent the HP to destruction (the EXISTING deplete_cover did the
    // bookkeeping — we only read its result, never re-pin a magnitude).
    let after = cover.peek(&at).copied();
    assert!(
        after.is_some(),
        "the struck cover entry must still be in the ledger after the hit",
    );
    let Some(after) = after else { return };
    assert!(
        *after.destroyed,
        "a destroyed piece must read destroyed == true"
    );
    assert_eq!(
        *after.current_hp, 0,
        "destruction empties the HP pool to zero"
    );

    // The cover path takes NO RNG draw (deterministic / replay-safe, C9).
    let mut rng_fresh = rng();
    assert_eq!(
        rng_used.next_u64(),
        rng_fresh.next_u64(),
        "a cover hit must take NO RNG draw",
    );
}

/// C8(b) — an INSUFFICIENT hit reduces cover HP WITHOUT destroying it and records NO
/// destroyed cell.
///
/// The cover's HP pool is large relative to the weapon's per-hit damage (which still
/// breaches the low protection), so one hit reduces HP but leaves the piece standing:
/// `deplete_cover` returns `Damaged`, the entry reads not-destroyed with HP strictly
/// between zero and its max, and the report carries `cover_destroyed == None`. Only the
/// reduction relation is asserted — no pinned magnitude.
#[test]
fn insufficient_hit_reduces_hp_without_destroying() {
    let tuning = CombatTuning::default();
    // Modest damage; a LARGE HP pool so one hit cannot empty it.
    let weapon = a_weapon(12, 6, 2, DamageType::Kinetic);
    let max_hp = 200_u32;
    let entry = cover_entry(max_hp, 2, 1);
    let at = cover_cell_level();

    let mut cover = ledger();
    cover.insert(at, entry);

    let mut rng_used = rng();
    let report = resolve_and_apply(
        &cover_outcome(entry),
        weapon.stats(),
        Luck::new(0.0),
        None,
        an_entity(),
        surfaces(&mut cover, &mut slab_ledger()),
        &tuning,
        &mut rng_used,
    );

    // No destroyed cell recorded — the piece still stands.
    assert_eq!(
        report.cover_destroyed, None,
        "an insufficient cover hit must record NO destroyed cell",
    );
    assert_eq!(report.applied, None, "a cover hit wounds no ganger");

    let after = cover.peek(&at).copied();
    assert!(
        after.is_some(),
        "the struck cover entry must still be in the ledger after the hit",
    );
    let Some(after) = after else { return };
    assert!(
        !*after.destroyed,
        "an insufficient hit must leave the piece NOT destroyed",
    );
    // HP fell (the hit breached the low protection) but the pool is not empty — the
    // reduction relation, never a pinned magnitude.
    assert!(
        *after.current_hp < max_hp,
        "an insufficient hit must REDUCE the cover HP below its max",
    );
    assert!(
        *after.current_hp > 0,
        "an insufficient hit must leave the cover HP above zero (not destroyed)",
    );

    // No RNG draw on the cover path (C9).
    let mut rng_fresh = rng();
    assert_eq!(
        rng_used.next_u64(),
        rng_fresh.next_u64(),
        "a cover hit must take NO RNG draw",
    );
}

/// C9 — the cover-hit fold is DETERMINISTIC under a seeded RNG: the same inputs
/// reproduce the same report AND the same depleted ledger state (and never perturb the
/// RNG stream, since the cover path takes no draw).
#[test]
fn cover_hit_is_deterministic_under_seeded_rng() {
    let run = || {
        let tuning = CombatTuning::default();
        let weapon = a_weapon(30, 20, 8, DamageType::Kinetic);
        let entry = cover_entry(50, 3, 2);
        let at = cover_cell_level();
        let mut cover = ledger();
        cover.insert(at, entry);
        let mut r = rng();
        let report = resolve_and_apply(
            &cover_outcome(entry),
            weapon.stats(),
            Luck::new(0.0),
            None,
            an_entity(),
            surfaces(&mut cover, &mut slab_ledger()),
            &tuning,
            &mut r,
        );
        (report, cover.peek(&at).copied())
    };

    assert_eq!(
        run(),
        run(),
        "the same seed + inputs must reproduce the same cover report AND ledger state",
    );
}
