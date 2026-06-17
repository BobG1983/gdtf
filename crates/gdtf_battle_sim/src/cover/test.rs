use super::{
    BandFraction, CoverDamage, CoverEntry, CoverEvent, CoverHp, CoverLedger, Destroyed, HeightBand,
    band_for,
};
use crate::{
    armor::{ArmorHardness, ArmorProtection},
    metric::{Cell, CellLevel, Level},
    tuning::CombatTuning,
};

/// An arbitrary prototype cover entry — NOT shipped magnitudes. Distinct
/// `max_hp` / band / armor so a test can prove the seed copies the prototype's
/// fields, while the arbitrary numbers keep the assertions on the MECHANISM
/// (lazy-seed / one-ledger / depletion), never a magnitude.
fn arbitrary_prototype(max_hp: u32, band: HeightBand) -> CoverEntry {
    CoverEntry::seeded(
        CoverHp::new(max_hp),
        band,
        ArmorProtection::new(7),
        ArmorHardness::new(3),
    )
}

fn key(x: i32, y: i32, level: u8) -> CellLevel {
    CellLevel::new(Cell::new(x, y), Level::new(level))
}

/// C8(a) — LAZY seeding: an absent entry, queried, returns a freshly seeded
/// entry with `current_hp == max_hp`.
///
/// The ledger starts empty (no key present), so `entry_seeded` on an untouched
/// key must seed `current_hp` to the prototype's `max_hp` and start
/// not-destroyed. Uses an arbitrary `max_hp` — the invariant is `current ==
/// max`, not a specific magnitude.
#[test]
fn absent_entry_seeds_current_to_max() {
    let mut ledger = CoverLedger::new();
    let k = key(2, 3, 1);
    let prototype = arbitrary_prototype(42, HeightBand::Mid);

    // The key is genuinely absent before the first access.
    assert!(
        ledger.peek(&k).is_none(),
        "ledger must start empty (no pre-seed)"
    );

    let seeded = ledger.entry_seeded(k, prototype);
    assert_eq!(
        seeded.current_hp, seeded.max_hp,
        "a freshly seeded entry must have current_hp == max_hp",
    );
    assert_eq!(
        seeded.max_hp, prototype.max_hp,
        "the seed must take its max_hp from the prototype",
    );
    assert_eq!(
        seeded.destroyed,
        Destroyed::new(false),
        "a freshly seeded entry is not destroyed",
    );
}

/// C8(b) — ONE-ledger invariant: a wall entry and a prop entry inserted into
/// the SAME map are both retrievable.
///
/// Walls and props share one `CoverLedger` (resolution.md §3); inserting one of
/// each on distinct `(cell, level)` keys and reading both back proves there is
/// no separate structure for either. The "wall" / "prop" distinction is purely
/// the caller's authored data (different bands here) — the map treats them
/// identically.
#[test]
fn wall_and_prop_share_one_ledger() {
    let mut ledger = CoverLedger::new();
    // A "wall": a tall, tough piece.
    let wall_key = key(0, 0, 0);
    let wall = arbitrary_prototype(100, HeightBand::High);
    // A "prop": a low, fragile piece, on a different key.
    let prop_key = key(5, 5, 0);
    let prop = arbitrary_prototype(20, HeightBand::Low);

    ledger.insert(wall_key, wall);
    ledger.insert(prop_key, prop);

    let got_wall = ledger.peek(&wall_key).copied();
    let got_prop = ledger.peek(&prop_key).copied();
    assert_eq!(got_wall, Some(wall), "the wall entry must be retrievable");
    assert_eq!(got_prop, Some(prop), "the prop entry must be retrievable");
    // And they are genuinely distinct entries in the one map.
    assert_ne!(
        got_wall, got_prop,
        "wall and prop are distinct entries in the SAME ledger",
    );
}

/// C8(c) — depletion sets `destroyed == true` EXACTLY when `current_hp` reaches
/// zero, and returns the [`CoverEvent::Destroyed`] marker carrying the
/// `(cell, level)`.
///
/// Reduces a freshly-seeded entry to zero in one hit and asserts both the
/// destroyed flag and the returned marker (with its `CellLevel`). Arbitrary
/// magnitudes — the mechanism is "reaches zero ⇒ destroyed + marker", not the
/// numbers.
#[test]
fn depletion_to_zero_destroys_and_returns_marker() {
    let mut ledger = CoverLedger::new();
    let tuning = CombatTuning::default();
    let k = key(4, 7, 2);
    let prototype = arbitrary_prototype(30, HeightBand::Low);

    // One hit exactly equal to max HP drives current_hp to zero.
    let event = ledger.deplete_cover(k, CoverDamage::new(30), prototype, &tuning);

    assert_eq!(
        event,
        CoverEvent::Destroyed(k),
        "reaching zero HP must return the Destroyed marker carrying (cell, level)",
    );
    // Re-query the stored entry (compared as `Some(..)`, never unwrapped — the
    // workspace denies unwrap/expect/panic in tests too).
    let stored = ledger.peek(&k).copied();
    assert_eq!(
        stored.map(|e| e.destroyed),
        Some(Destroyed::new(true)),
        "current_hp reaching zero must set destroyed = true",
    );
    assert_eq!(
        stored.map(|e| *e.current_hp),
        Some(0),
        "destroyed cover has current_hp == 0",
    );
}

/// C8(c) the other side — a PARTIAL hit (not to zero) does NOT destroy and
/// returns the [`CoverEvent::Damaged`] marker.
///
/// Pins that `destroyed` flips EXACTLY at zero, not before: a hit smaller than
/// `max_hp` leaves the cover standing with reduced HP and an un-set flag.
#[test]
fn partial_depletion_does_not_destroy() {
    let mut ledger = CoverLedger::new();
    let tuning = CombatTuning::default();
    let k = key(1, 1, 0);
    let prototype = arbitrary_prototype(50, HeightBand::Mid);

    let event = ledger.deplete_cover(k, CoverDamage::new(20), prototype, &tuning);

    assert_eq!(
        event,
        CoverEvent::Damaged(k),
        "a partial hit must return the Damaged marker, not Destroyed",
    );
    let stored = ledger.peek(&k).copied();
    assert_eq!(
        stored.map(|e| e.destroyed),
        Some(Destroyed::new(false)),
        "a partial hit must NOT set destroyed",
    );
    assert_eq!(
        stored.map(|e| *e.current_hp),
        Some(30),
        "current_hp must be max_hp - damage after a partial hit",
    );
}

/// A second hit that finishes a previously-damaged piece destroys it — the
/// ledger persists `current_hp` across hits (it is not re-seeded each call).
#[test]
fn second_hit_finishes_a_damaged_piece() {
    let mut ledger = CoverLedger::new();
    let tuning = CombatTuning::default();
    let k = key(3, 3, 1);
    let prototype = arbitrary_prototype(10, HeightBand::Low);

    let first = ledger.deplete_cover(k, CoverDamage::new(6), prototype, &tuning);
    assert_eq!(first, CoverEvent::Damaged(k));

    // The second hit reads the persisted (reduced) HP, not a fresh seed.
    let second = ledger.deplete_cover(k, CoverDamage::new(6), prototype, &tuning);
    assert_eq!(
        second,
        CoverEvent::Destroyed(k),
        "the second hit must finish the already-damaged piece",
    );
}

/// [`band_for`] classifies within-level fractions using ONLY the tuning edges
/// — no hardcoded band numbers (C6). Drives a fraction below, between, and
/// above the tuning's two edges and asserts LOW / MID / HIGH, deriving the
/// probe fractions from the edges themselves (so a tuning edit moves the
/// boundaries with it).
#[test]
fn band_for_reads_tuning_edges() {
    let tuning = CombatTuning::default();
    let edges = &tuning.projectile_band_edges;

    // Strictly below the LOW→MID edge → LOW (half the edge is below it for any
    // positive edge, deriving the probe from the edge, not a literal).
    let low = band_for(BandFraction::new(*edges.low_mid * 0.5), &tuning);
    assert_eq!(low, HeightBand::Low);
    // At-or-above LOW→MID but below MID→HIGH → MID.
    let mid = band_for(BandFraction::new(*edges.low_mid), &tuning);
    assert_eq!(mid, HeightBand::Mid);
    // At-or-above the MID→HIGH edge → HIGH.
    let high = band_for(BandFraction::new(*edges.mid_high), &tuning);
    assert_eq!(high, HeightBand::High);
}

/// The `from_band` round-trip lands back in the same band under `band_for` for
/// all three bands — proving `deplete_cover`'s tuning re-classification is a
/// no-op on an already-correct band (it never drifts a stored band).
#[test]
fn band_round_trips_through_tuning() {
    let tuning = CombatTuning::default();
    for band in [HeightBand::Low, HeightBand::Mid, HeightBand::High] {
        let fraction = BandFraction::from_band(band, &tuning);
        assert_eq!(
            band_for(fraction, &tuning),
            band,
            "{band:?} must round-trip"
        );
    }
}

/// The cover newtypes' derived [`Deref`] reaches their inner value. Built from
/// arbitrary literals — pins the Deref mechanism, not a magnitude.
#[test]
fn cover_newtypes_deref_to_inner() {
    assert_eq!(*CoverHp::new(13), 13u32);
    assert_eq!(*CoverDamage::new(4), 4u32);
    assert!(!*Destroyed::new(false));
    assert!(*Destroyed::new(true));
    assert_eq!((*BandFraction::new(5.0)).to_bits(), 5.0_f32.to_bits());
}
