use super::support::*;

/// AC6 (no-bare-types / frozen) — the report and its block are `Copy` records of
/// named domain newtypes (no bare primitive), and the report is freely copyable.
/// Pins the frozen-record shape (mechanism), never a tuning magnitude.
#[test]
fn report_is_a_frozen_record_of_named_newtypes() {
    let entity = an_entity();
    // A no-effect report copies and compares by value.
    let report = HitReport::no_effect(ShotKind::Miss);
    let copied = report; // Copy, not a move
    assert_eq!(report, copied, "HitReport must be Copy + PartialEq");
    assert_eq!(report.applied, None);
    assert_eq!(report.part, None);

    // An applied block is a Copy record of named newtypes.
    let applied = AppliedDamage {
        matchup:    Matchup::Favorable,
        hit:        HitResult {
            penetrating: crate::resolve_hit::PenetratingDamage::new(7),
            hp_damage:   crate::resolve_hit::HpDamage::new(8),
            wear:        crate::resolve_hit::IntegrityWear::new(9),
        },
        severity:   Severity::Major,
        life_after: LifeState::Downed,
        broken:     Some(ArmorBroken::new(entity, BodyPart::Torso)),
    };
    let applied_copy = applied; // Copy
    assert_eq!(
        applied, applied_copy,
        "AppliedDamage must be Copy + PartialEq"
    );
    // The block's fields are the named domain types (Deref reaches their inner).
    assert_eq!(*applied.hit.hp_damage, 8i32);
    assert_eq!(applied.severity, Severity::Major);
    assert_eq!(applied.matchup, Matchup::Favorable);
}
