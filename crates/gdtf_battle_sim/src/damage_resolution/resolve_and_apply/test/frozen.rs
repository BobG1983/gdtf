use super::support::*;

#[test]
fn report_is_a_frozen_record_of_named_newtypes() {
    let entity = an_entity();
    let report = HitReport::no_effect(ShotKind::Miss);
    let cloned = report.clone();
    assert_eq!(report, cloned, "HitReport must be Clone + PartialEq");
    assert_eq!(report.verdict, HitVerdict::NoEffect);

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
    let applied_copy = applied; 
    assert_eq!(
        applied, applied_copy,
        "AppliedDamage must be Copy + PartialEq"
    );
    assert_eq!(*applied.hit.hp_damage, 8i32);
    assert_eq!(applied.severity, Severity::Major);
    assert_eq!(applied.matchup, Matchup::Favorable);

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
