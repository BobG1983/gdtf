use super::{SlabDamage, SlabEntry, SlabEvent, SlabHp, SlabLedger};
use crate::{
    armor::{ArmorHardness, ArmorProtection},
    metric::{Cell, CellLevel, Level},
};

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

#[test]
fn absent_entry_is_never_minted_by_depletion() {
    let mut ledger = SlabLedger::new();
    let k = key(2, 3, 1);

    let event = ledger.deplete_slab(k, SlabDamage::new(30));
    assert_eq!(
        event, None,
        "depleting a cell with no authored slab must answer None, not {event:?}"
    );
    let after = ledger.peek(&k).copied();
    assert!(
        after.is_none(),
        "depletion must mint no entry at a never-inserted cell; found {after:?}"
    );
}

#[test]
fn one_non_lethal_hit_damages_and_records() {
    let mut ledger = SlabLedger::new();
    let k = key(4, 4, 0);
    ledger.insert(k, arbitrary_prototype(100));
    let event = ledger.deplete_slab(k, SlabDamage::new(30));
    assert_eq!(
        event,
        Some(SlabEvent::Damaged(k)),
        "a non-lethal hit must report Damaged, not Destroyed"
    );
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

#[test]
fn multiple_persistent_hits_eventually_destroy() {
    let mut ledger = SlabLedger::new();
    let k = key(7, 1, 2);
    ledger.insert(k, arbitrary_prototype(100));
    let per_hit = SlabDamage::new(25);

    for strike in 1..=3 {
        let event = ledger.deplete_slab(k, per_hit);
        assert_eq!(
            event,
            Some(SlabEvent::Damaged(k)),
            "strike {strike} (sub-lethal) must Damage, proving the pool persists across hits"
        );
    }
    let event = ledger.deplete_slab(k, per_hit);
    assert_eq!(
        event,
        Some(SlabEvent::Destroyed(k)),
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

#[test]
fn one_overkill_hit_destroys_and_saturates_at_zero() {
    let mut ledger = SlabLedger::new();
    let k = key(9, 9, 0);
    ledger.insert(k, arbitrary_prototype(50));
    let event = ledger.deplete_slab(k, SlabDamage::new(9_999));
    assert_eq!(
        event,
        Some(SlabEvent::Destroyed(k)),
        "an over-killing hit must report Destroyed"
    );
    assert!(
        ledger.peek(&k).copied().is_some_and(|e| *e.current_hp == 0),
        "over-kill must saturate the HP pool at zero, never below (and the slab is present)"
    );
}
