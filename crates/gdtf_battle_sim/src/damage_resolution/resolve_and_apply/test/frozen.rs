use super::support::*;

/// AC6 (no-bare-types / frozen) — the report and its verdict are frozen records of
/// named domain newtypes (no bare primitive). The report is [`Clone`] + `PartialEq`
/// (NOT `Copy` since GTW-438: a ganger verdict carries the rolled
/// `Option<RolledInjury>`, an owned `Vec` + texts — boxed inside
/// [`HitVerdict::Ganger`] since GTW-573); its [`AppliedDamage`] block stays `Copy`,
/// carrying the closed [`ArmorWearOutcome`] directly (GTW-573 C2 — never a
/// prose-exclusive `broken`/`worn` Option pair). Pins the frozen-record shape
/// (mechanism), never a tuning magnitude.
#[test]
fn report_is_a_frozen_record_of_named_newtypes() {
    let entity = an_entity();
    // A no-effect report clones and compares by value (Clone, not Copy since GTW-438).
    let report = HitReport::no_effect(ShotKind::Miss);
    let cloned = report.clone();
    assert_eq!(report, cloned, "HitReport must be Clone + PartialEq");
    // The no-effect verdict is the closed enum's inert variant — no parallel per-kind
    // Options exist to zero out (GTW-573 C1).
    assert_eq!(report.verdict, HitVerdict::NoEffect);

    // An applied block is a Copy record of named newtypes, its wear outcome the CLOSED
    // ArmorWearOutcome enum (a Broke-AND-Damaged state is unrepresentable).
    let applied = AppliedDamage {
        matchup:    Matchup::Favorable,
        hit:        HitResult {
            penetrating: crate::resolve_hit::PenetratingDamage::new(7),
            hp_damage:   crate::resolve_hit::HpDamage::new(8),
            wear:        crate::resolve_hit::IntegrityWear::new(9),
        },
        severity:   Severity::Major,
        life_after: LifeState::Downed,
        wear:       ArmorWearOutcome::Broke(ArmorBroken::new(entity, BodyPart::Torso)),
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

    // A ganger verdict freezes the whole wound payload behind one box; equality is
    // by-value (the seeded-replay comparison shape).
    let verdict = HitVerdict::Ganger(Box::new(GangerVerdict {
        target: entity,
        part: BodyPart::Torso,
        applied,
        injury: None,
        dot_applied: None,
    }));
    assert_eq!(
        verdict.clone(),
        verdict,
        "a ganger verdict must be Clone + PartialEq"
    );
}
