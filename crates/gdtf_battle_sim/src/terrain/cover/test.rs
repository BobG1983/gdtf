use super::{
    BandFraction, CoverDamage, CoverEntry, CoverEvent, CoverHp, CoverLedger, Destroyed, HeightBand,
    band_for,
};
use crate::{
    armor::{ArmorHardness, ArmorProtection},
    metric::{Cell, CellLevel, Level},
    tuning::CombatTuning,
};

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

#[test]
fn absent_entry_seeds_current_to_max() {
    let mut ledger = CoverLedger::new();
    let k = key(2, 3, 1);
    let prototype = arbitrary_prototype(42, HeightBand::Mid);

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

#[test]
fn wall_and_prop_share_one_ledger() {
    let mut ledger = CoverLedger::new();
    let wall_key = key(0, 0, 0);
    let wall = arbitrary_prototype(100, HeightBand::High);
    let prop_key = key(5, 5, 0);
    let prop = arbitrary_prototype(20, HeightBand::Low);

    ledger.insert(wall_key, wall);
    ledger.insert(prop_key, prop);

    let got_wall = ledger.peek(&wall_key).copied();
    let got_prop = ledger.peek(&prop_key).copied();
    assert_eq!(got_wall, Some(wall), "the wall entry must be retrievable");
    assert_eq!(got_prop, Some(prop), "the prop entry must be retrievable");
    assert_ne!(
        got_wall, got_prop,
        "wall and prop are distinct entries in the SAME ledger",
    );
}

#[test]
fn depletion_to_zero_destroys_and_returns_marker() {
    let mut ledger = CoverLedger::new();
    let tuning = CombatTuning::default();
    let k = key(4, 7, 2);
    let prototype = arbitrary_prototype(30, HeightBand::Low);

    let event = ledger.deplete_cover(k, CoverDamage::new(30), prototype, &tuning);

    assert_eq!(
        event,
        CoverEvent::Destroyed(k),
        "reaching zero HP must return the Destroyed marker carrying (cell, level)",
    );
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

#[test]
fn second_hit_finishes_a_damaged_piece() {
    let mut ledger = CoverLedger::new();
    let tuning = CombatTuning::default();
    let k = key(3, 3, 1);
    let prototype = arbitrary_prototype(10, HeightBand::Low);

    let first = ledger.deplete_cover(k, CoverDamage::new(6), prototype, &tuning);
    assert_eq!(first, CoverEvent::Damaged(k));

    let second = ledger.deplete_cover(k, CoverDamage::new(6), prototype, &tuning);
    assert_eq!(
        second,
        CoverEvent::Destroyed(k),
        "the second hit must finish the already-damaged piece",
    );
}

#[test]
fn band_for_reads_tuning_edges() {
    let tuning = CombatTuning::default();
    let edges = &tuning.projectile_band_edges;

    let low = band_for(BandFraction::new(*edges.low_mid * 0.5), &tuning);
    assert_eq!(low, HeightBand::Low);
    let mid = band_for(BandFraction::new(*edges.low_mid), &tuning);
    assert_eq!(mid, HeightBand::Mid);
    let high = band_for(BandFraction::new(*edges.mid_high), &tuning);
    assert_eq!(high, HeightBand::High);
}

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

#[test]
fn cover_newtypes_deref_to_inner() {
    assert_eq!(*CoverHp::new(13), 13u32);
    assert_eq!(*CoverDamage::new(4), 4u32);
    assert!(!*Destroyed::new(false));
    assert!(*Destroyed::new(true));
    assert_eq!((*BandFraction::new(5.0)).to_bits(), 5.0_f32.to_bits());
}
