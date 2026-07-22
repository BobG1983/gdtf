//! Unit tests for the [`SlabLedger`] — lazy-seed, PERSISTENT depletion across strikes,
//! and the destruction marker (C4 / the slab mirror of the cover-ledger tests). All
//! value-agnostic: arbitrary HP/damage, never a shipped magnitude.

use super::{SlabDamage, SlabEntry, SlabEvent, SlabHp, SlabLedger};
use crate::{
    armor::{ArmorHardness, ArmorProtection},
    metric::{Cell, CellLevel, Level},
    tuning::SlabDefaults,
};

/// An arbitrary prototype slab entry — NOT shipped magnitudes. Distinct `max_hp` /
/// armor so a test can prove the seed copies the prototype's fields, while the
/// arbitrary numbers keep the assertions on the MECHANISM (lazy-seed / persistence /
/// destruction), never a magnitude.
fn arbitrary_prototype(max_hp: u32) -> SlabEntry {
    SlabEntry::seeded(
        SlabHp::new(max_hp),
        ArmorProtection::new(5),
        ArmorHardness::new(2),
    )
}

fn key(x: i32, y: i32, level: u8) -> CellLevel {
    CellLevel::new(Cell::new(x, y), Level::new(level))
}

/// LAZY seeding: an absent entry, queried, returns a freshly seeded entry with
/// `current_hp == max_hp` and not destroyed.
#[test]
fn absent_entry_seeds_current_to_max() {
    let mut ledger = SlabLedger::new();
    let k = key(2, 3, 1);
    let prototype = arbitrary_prototype(42);
    let entry = ledger.entry_seeded(k, prototype);
    assert_eq!(
        entry.current_hp, entry.max_hp,
        "a freshly-seeded slab must start at full HP (current == max)"
    );
    assert!(
        !*entry.destroyed,
        "a freshly-seeded slab must not be destroyed"
    );
}

/// A single non-lethal hit DAMAGES the slab (still standing) — and the new, reduced HP
/// is recorded in the map (persistence groundwork for the multi-hit test).
#[test]
fn one_non_lethal_hit_damages_and_records() {
    let mut ledger = SlabLedger::new();
    let k = key(4, 4, 0);
    let prototype = arbitrary_prototype(100);
    // Spend less than max, so the slab survives.
    let event = ledger.deplete_slab(k, SlabDamage::new(30), prototype);
    assert_eq!(
        event,
        SlabEvent::Damaged(k),
        "a non-lethal hit must report Damaged, not Destroyed"
    );
    // The reduced HP is in the map (peek does NOT re-seed), and it is strictly below max.
    let after = ledger.peek(&k).copied();
    assert!(
        after.is_some_and(|e| *e.current_hp < *e.max_hp),
        "a damaged slab must be present with current HP strictly below max"
    );
    assert!(
        after.is_some_and(|e| !*e.destroyed),
        "a damaged slab must not be destroyed"
    );
}

/// C4 — a slab depletes over MULTIPLE PERSISTENT strikes: a sequence of sub-lethal hits
/// that each leave the slab standing eventually drives `current_hp` to zero, at which
/// point the ledger reports `Destroyed`. The HP pool is persistent (each hit reads the
/// reduced total left by the prior one) — proved by needing several hits, not one.
#[test]
fn multiple_persistent_hits_eventually_destroy() {
    let mut ledger = SlabLedger::new();
    let k = key(7, 1, 2);
    let prototype = arbitrary_prototype(100);
    // A per-hit damage that is a fraction of max, so NO single hit destroys it.
    let per_hit = SlabDamage::new(25);

    // The first three hits must each only DAMAGE (the pool persists below max, never zero).
    for strike in 1..=3 {
        let event = ledger.deplete_slab(k, per_hit, prototype);
        assert_eq!(
            event,
            SlabEvent::Damaged(k),
            "strike {strike} (sub-lethal) must Damage, proving the pool persists across hits"
        );
    }
    // The fourth hit spends the last quarter — the persistent pool hits zero → Destroyed.
    let event = ledger.deplete_slab(k, per_hit, prototype);
    assert_eq!(
        event,
        SlabEvent::Destroyed(k),
        "the strike that drains the PERSISTENT pool to zero must report Destroyed"
    );
    let after = ledger.peek(&k).copied();
    assert!(
        after.is_some_and(|e| *e.current_hp == 0),
        "a destroyed slab must still be present with current HP zero"
    );
    assert!(
        after.is_some_and(|e| *e.destroyed),
        "a slab drained to zero must carry the destroyed flag"
    );
}

/// A single OVER-killing hit destroys the slab in one strike (HP saturates at zero, not
/// below) — the boundary the multi-hit test complements.
#[test]
fn one_overkill_hit_destroys_and_saturates_at_zero() {
    let mut ledger = SlabLedger::new();
    let k = key(9, 9, 0);
    let prototype = arbitrary_prototype(50);
    // Spend MORE than max — the pool saturates at zero, never wraps below.
    let event = ledger.deplete_slab(k, SlabDamage::new(9_999), prototype);
    assert_eq!(
        event,
        SlabEvent::Destroyed(k),
        "an over-killing hit must report Destroyed"
    );
    assert!(
        ledger.peek(&k).copied().is_some_and(|e| *e.current_hp == 0),
        "over-kill must saturate the HP pool at zero, never below (and the slab is present)"
    );
}

/// The C7 tuning read is GENUINELY CONSUMED: `prototype_for` builds the lazy-seed prototype
/// from the [`SlabDefaults`] tuning leaf (not a hardcoded value), so depleting an
/// untouched slab seeds its `max_hp` / armor from the tuning. Value-agnostic — it
/// asserts the prototype mirrors the tuning leaf, never a shipped magnitude.
#[test]
fn prototype_for_reads_the_tuning_defaults() {
    let defaults = SlabDefaults::default();
    let k = key(1, 1, 1);
    let prototype = SlabLedger::prototype_for(k, &defaults);
    assert_eq!(
        prototype.max_hp,
        defaults.hp(),
        "the lazy-seed prototype's max_hp must come from the SlabDefaults tuning leaf (C7)"
    );
    assert_eq!(
        prototype.armor_protection,
        defaults.armor_protection(),
        "the lazy-seed prototype's armor must come from the SlabDefaults tuning leaf (C7)"
    );
    assert_eq!(
        prototype.armor_hardness,
        defaults.armor_hardness(),
        "the lazy-seed prototype's hardness must come from the SlabDefaults tuning leaf (C7)"
    );
    assert_eq!(
        prototype.current_hp, prototype.max_hp,
        "a freshly-seeded prototype starts at full HP"
    );
}
