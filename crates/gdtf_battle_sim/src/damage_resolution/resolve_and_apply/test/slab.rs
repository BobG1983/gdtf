//! GTW-365 — the slab-hit path through the REAL fold
//! ([`resolve_and_apply`](super::super::resolve_and_apply)): a `ShotKind::Slab`
//! outcome reuses the ganger damage formula against the slab's OWN armor, spends the
//! [`SlabLedger`](crate::slab::SlabLedger)'s HP via `deplete_slab`, and records a
//! destroyed `(cell, level)` on the report ONLY when the hit depletes the HP to zero —
//! the slab mirror of [`super::cover`].
//!
//! These assert the MECHANISM, not magnitudes (C7 / C9): the relations "HP fell to
//! destruction + a destroyed cell was recorded" (a SUFFICIENT hit) and "HP fell but the
//! slab still stands + no cell recorded" (an INSUFFICIENT hit) — never a pinned HP
//! number. A fixture pre-inserts a slab entry at the struck key so the fold depletes a
//! known pool (lazy-seed only fills an ABSENT key), letting the test choose the regime.

use super::support::*;

/// C9(a) — a SUFFICIENT hit depletes slab HP to destruction AND records the destroyed
/// `(cell, level)` on the report (`report.slab_destroyed == Some(the struck slab)`).
///
/// The weapon's damage far exceeds the slab's (low) armor + (small) HP, so the reused
/// `resolve_hit` damage spends the ledger's HP to zero: `deplete_slab` returns
/// `Destroyed`, the entry reads destroyed at zero HP, and the report carries the
/// destroyed slab key. Magnitudes are NOT pinned — only the destruction relation is.
#[test]
fn sufficient_hit_destroys_slab_and_records_the_cell() {
    let tuning = CombatTuning::default();
    // High damage, low slab armor + small HP → the hit breaches and empties the pool.
    let weapon = a_weapon(60, 40, 20, DamageType::Kinetic);
    let at = slab_cell_level();

    let mut slab = slab_ledger();
    // Pre-insert a low-HP entry at the struck key so the fold depletes a known pool
    // (lazy-seed only seeds an ABSENT key; an inserted entry is used as-is).
    slab.insert(at, slab_entry(20, 2, 1));
    let mut cover = ledger();

    let mut rng_used = rng();
    let report = resolve_and_apply(
        &slab_outcome(),
        weapon.stats(),
        Luck::new(0.0),
        // No struck ganger on a slab hit.
        None,
        an_entity(),
        surfaces(&mut cover, &mut slab),
        &tuning,
        &mut rng_used,
        &injury_tables(),
        &injury_registry(),
        &mut injury_rng(),
    );

    // The verdict IS the slab kind's record (a ganger wound / cover destruction is
    // structurally impossible on it — GTW-573), carrying the destroyed slab the bridge
    // dispatch_fire turns into a SlabDestroyed message.
    assert_eq!(
        report.verdict,
        HitVerdict::Slab(SlabVerdict {
            destroyed: Some(at),
        }),
        "a sufficient slab hit must record the destroyed (cell, level) on the verdict",
    );

    // The ledger spent the HP to destruction (the EXISTING deplete_slab did the
    // bookkeeping — we only read its result, never re-pin a magnitude).
    let after = slab.peek(&at).copied();
    assert!(
        after.is_some_and(|e| *e.destroyed && *e.current_hp == 0),
        "a destroyed slab must read destroyed == true at zero HP",
    );

    // The slab path takes NO RNG draw (deterministic / replay-safe, C9).
    let mut rng_fresh = rng();
    assert_eq!(
        rng_used.next_u64(),
        rng_fresh.next_u64(),
        "a slab hit must take NO RNG draw",
    );
}

/// C9(a) — an INSUFFICIENT hit reduces slab HP WITHOUT destroying it and records NO
/// destroyed cell. (The persistence/multi-hit MECHANISM is also unit-tested in
/// [`crate::slab`]'s ledger tests; this proves it through the REAL fold.)
#[test]
fn insufficient_hit_reduces_hp_without_destroying() {
    let tuning = CombatTuning::default();
    // Modest damage; a LARGE HP pool so one hit cannot empty it.
    let weapon = a_weapon(12, 6, 2, DamageType::Kinetic);
    let max_hp = 400_u32;
    let at = slab_cell_level();

    let mut slab = slab_ledger();
    slab.insert(at, slab_entry(max_hp, 2, 1));
    let mut cover = ledger();

    let mut rng_used = rng();
    let report = resolve_and_apply(
        &slab_outcome(),
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

    // A REAL slab verdict with no destroyed cell — the slab still stands (and a ganger
    // wound is structurally impossible on a slab verdict).
    assert_eq!(
        report.verdict,
        HitVerdict::Slab(SlabVerdict { destroyed: None }),
        "an insufficient slab hit must record NO destroyed cell",
    );

    let after = slab.peek(&at).copied();
    // HP fell (the hit breached the low armor) but the pool is not empty — the reduction
    // relation, never a pinned magnitude.
    assert!(
        after.is_some_and(|e| !*e.destroyed && *e.current_hp < max_hp && *e.current_hp > 0),
        "an insufficient hit must REDUCE the slab HP below max but leave it above zero",
    );

    // No RNG draw on the slab path (C9).
    let mut rng_fresh = rng();
    assert_eq!(
        rng_used.next_u64(),
        rng_fresh.next_u64(),
        "a slab hit must take NO RNG draw",
    );
}

/// C9 — the slab-hit fold is DETERMINISTIC under a seeded RNG: the same inputs reproduce
/// the same report AND the same depleted ledger state (and never perturb the RNG stream,
/// since the slab path takes no draw).
#[test]
fn slab_hit_is_deterministic_under_seeded_rng() {
    let run = || {
        let tuning = CombatTuning::default();
        let weapon = a_weapon(30, 20, 8, DamageType::Kinetic);
        let at = slab_cell_level();
        let mut slab = slab_ledger();
        slab.insert(at, slab_entry(50, 3, 2));
        let mut cover = ledger();
        let mut r = rng();
        let report = resolve_and_apply(
            &slab_outcome(),
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
        (report, slab.peek(&at).copied())
    };

    assert_eq!(
        run(),
        run(),
        "the same seed + inputs must reproduce the same slab report AND ledger state",
    );
}
