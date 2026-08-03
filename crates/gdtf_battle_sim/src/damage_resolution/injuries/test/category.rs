use super::{
    super::{
        DamageContext, HandsAvailable, InflictedInjuries, InjuryDef, InjuryEffect, InjuryName,
        InjuryRegistry, InjuryTables, InjuryWeight, InspectText, LogText, MovementCostFactor,
        PopupText, PostHeal, WeightedInjuryEntry, WeightedInjuryTable, roll_injury,
    },
    support::*,
};
use crate::{
    armor::{BodyPart, InjuryCategory},
    severity::Severity,
};

fn shared_arm_table(key: &str) -> (InjuryRegistry, InjuryTables) {
    let name = InjuryName::new(key.to_owned());
    let def = InjuryDef {
        name:         name.clone(),
        category:     InjuryCategory::Arm,
        severity:     Severity::Major,
        popup_text:   PopupText::new("HAND SHATTERED".to_owned()),
        log_text:     LogText::new("shatters a hand".to_owned()),
        inspect_text: InspectText::new("Shattered Hand".to_owned()),
        effects:      vec![InjuryEffect::DisableHand],
        post_heal:    PostHeal::Deferred,
    };
    let registry = InjuryRegistry::new([(name.clone(), def)]);
    let mut tables = InjuryTables::default();
    tables.insert(
        InjuryCategory::Arm,
        DamageContext::Ranged,
        Severity::Major,
        WeightedInjuryTable::new(vec![WeightedInjuryEntry::new(name, InjuryWeight::new(10))]),
    );
    (registry, tables)
}

#[test]
fn left_and_right_arm_sample_the_same_shared_arm_pool() {
    let (registry, tables) = shared_arm_table("shattered_hand");

    let mut rng_left = injury_rng();
    let left = roll_injury(
        BodyPart::LeftArm,
        Severity::Major,
        DamageContext::Ranged,
        &tables,
        &registry,
        &mut rng_left,
    );
    let mut rng_right = injury_rng();
    let right = roll_injury(
        BodyPart::RightArm,
        Severity::Major,
        DamageContext::Ranged,
        &tables,
        &registry,
        &mut rng_right,
    );

    assert!(
        left.is_some(),
        "a LeftArm Major roll must sample the shared Arm pool"
    );
    assert!(
        right.is_some(),
        "a RightArm Major roll must sample the SAME shared Arm pool (per-category resolution)",
    );
    assert_eq!(
        left.as_ref().map(|r| r.name.clone()),
        Some(InjuryName::new("shattered_hand".to_owned())),
    );
    assert_eq!(
        right.as_ref().map(|r| r.name.clone()),
        left.as_ref().map(|r| r.name.clone()),
        "LeftArm and RightArm draw the identical shared-pool injury",
    );
}

#[test]
fn disable_hand_side_comes_from_struck_part_not_the_file() {
    let (registry, tables) = shared_arm_table("shattered_hand");

    let mut rng_left = injury_rng();
    let left_pick = roll_injury(
        BodyPart::LeftArm,
        Severity::Major,
        DamageContext::Ranged,
        &tables,
        &registry,
        &mut rng_left,
    );
    let mut rng_right = injury_rng();
    let right_pick = roll_injury(
        BodyPart::RightArm,
        Severity::Major,
        DamageContext::Ranged,
        &tables,
        &registry,
        &mut rng_right,
    );
    assert!(
        left_pick.is_some() && right_pick.is_some(),
        "the shared Arm pool must roll its catalog entry for BOTH arms",
    );
    let (Some(left_rolled), Some(right_rolled)) = (left_pick, right_pick) else {
        return;
    };

    assert_eq!(
        left_rolled.part,
        BodyPart::LeftArm,
        "the rolled verdict's part is the struck LEFT arm",
    );
    assert_eq!(
        right_rolled.part,
        BodyPart::RightArm,
        "the rolled verdict's part is the struck RIGHT arm (not the def's side-agnostic Arm category)",
    );

    let mut left_only = InflictedInjuries::default();
    left_only.gain(left_rolled.into_gained());
    assert_eq!(
        left_only.hands_available(),
        HandsAvailable::new(1),
        "a LeftArm DisableHand disables exactly one (the left) hand",
    );

    let mut both = InflictedInjuries::default();
    both.gain(left_only.gained()[0].clone());
    both.gain(right_rolled.into_gained());
    assert_eq!(
        both.hands_available(),
        HandsAvailable::new(0),
        "a LeftArm + a RightArm DisableHand (both from the shared Arm pool) disable BOTH \
         hands — the side comes from the struck part, not the def",
    );
}

#[test]
fn leg_movement_cost_mul_yields_a_hampered_factor_above_one() {
    let name = InjuryName::new("hampered_leg".to_owned());
    let def = InjuryDef {
        name:         name.clone(),
        category:     InjuryCategory::Leg,
        severity:     Severity::Major,
        popup_text:   PopupText::new("HAMPERED".to_owned()),
        log_text:     LogText::new("is hampered".to_owned()),
        inspect_text: InspectText::new("Hampered".to_owned()),
        effects:      vec![InjuryEffect::MovementCostMul(MovementCostFactor::new(1.5))],
        post_heal:    PostHeal::Deferred,
    };
    let registry = InjuryRegistry::new([(name.clone(), def)]);
    let mut tables = InjuryTables::default();
    tables.insert(
        InjuryCategory::Leg,
        DamageContext::Ranged,
        Severity::Major,
        WeightedInjuryTable::new(vec![WeightedInjuryEntry::new(name, InjuryWeight::new(10))]),
    );

    let mut rng = injury_rng();
    let pick = roll_injury(
        BodyPart::RightLeg,
        Severity::Major,
        DamageContext::Ranged,
        &tables,
        &registry,
        &mut rng,
    );
    assert!(
        pick.is_some(),
        "the shared Leg pool must roll its catalog entry"
    );
    let Some(rolled) = pick else {
        return;
    };
    let mut ledger = InflictedInjuries::default();
    ledger.gain(rolled.into_gained());

    assert!(
        ledger.movement_cost_factor().raw() > 1.0,
        "a leg MovementCostMul must yield a Hampered factor > 1.0 (got {:?})",
        ledger.movement_cost_factor(),
    );
}
