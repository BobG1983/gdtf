use super::{
    super::{DamageContext, InjuryRegistry, InjuryTables, roll_injury},
    support::*,
};
use crate::{armor::BodyPart, severity::Severity};

#[test]
fn roll_injury_takes_no_draw_for_none_or_fatal() {
    let (registry, tables) = one_injury_table("hurt", BodyPart::Head, Severity::Minor);
    for severity in [Severity::None, Severity::Fatal] {
        let mut rng = injury_rng();
        let rolled = roll_injury(
            BodyPart::Head,
            severity,
            DamageContext::Ranged,
            &tables,
            &registry,
            &mut rng,
        );
        assert!(
            rolled.is_none(),
            "{severity:?} is not tabled — no injury rolled"
        );
        assert_eq!(
            rng.next_u64(),
            injury_rng().next_u64(),
            "{severity:?} must take NO InjuryRng draw (the cursor stays put)"
        );
    }
}

#[test]
fn roll_injury_draws_exactly_one_and_resolves_a_tabled_severity() {
    let (registry, tables) = one_injury_table("hurt", BodyPart::Torso, Severity::Major);
    let mut rng = injury_rng();
    let rolled = roll_injury(
        BodyPart::Torso,
        Severity::Major,
        DamageContext::Ranged,
        &tables,
        &registry,
        &mut rng,
    );
    assert!(
        rolled.is_some(),
        "a populated Major bucket must roll an injury"
    );
    let Some(rolled) = rolled else {
        return;
    };
    assert_eq!(rolled.part, BodyPart::Torso);
    assert_eq!(rolled.severity, Severity::Major);
    assert_eq!(
        rolled.effects.len(),
        1,
        "the def's single effect is frozen on"
    );

    let mut fresh = injury_rng();
    let _ = fresh.next_u64();
    assert_eq!(
        rng.next_u64(),
        fresh.next_u64(),
        "a tabled roll must take EXACTLY ONE InjuryRng draw"
    );
}

#[test]
fn roll_injury_empty_table_draws_then_discards_content_independent() {
    let empty_registry = InjuryRegistry::default();
    let empty_tables = InjuryTables::default();
    let (full_registry, full_tables) = one_injury_table("hurt", BodyPart::Head, Severity::Minor);

    let mut rng_empty = injury_rng();
    let empty = roll_injury(
        BodyPart::Head,
        Severity::Minor,
        DamageContext::Ranged,
        &empty_tables,
        &empty_registry,
        &mut rng_empty,
    );
    assert!(empty.is_none(), "an empty bucket rolls no injury");

    let mut rng_full = injury_rng();
    let full = roll_injury(
        BodyPart::Head,
        Severity::Minor,
        DamageContext::Ranged,
        &full_tables,
        &full_registry,
        &mut rng_full,
    );
    assert!(full.is_some(), "a populated bucket rolls an injury");

    assert_eq!(
        rng_empty.next_u64(),
        rng_full.next_u64(),
        "the InjuryRng cursor must end at the SAME position for an empty vs a populated \
         bucket — the one draw is content-independent (stream alignment)"
    );
}

#[test]
fn roll_injury_is_deterministic_for_a_fixed_seed() {
    let (registry, tables) = one_injury_table("hurt", BodyPart::LeftLeg, Severity::Critical);
    let mut rng_a = injury_rng();
    let mut rng_b = injury_rng();
    let a = roll_injury(
        BodyPart::LeftLeg,
        Severity::Critical,
        DamageContext::Ranged,
        &tables,
        &registry,
        &mut rng_a,
    );
    let b = roll_injury(
        BodyPart::LeftLeg,
        Severity::Critical,
        DamageContext::Ranged,
        &tables,
        &registry,
        &mut rng_b,
    );
    assert_eq!(a, b, "the same seed must reproduce the same rolled injury");
}
