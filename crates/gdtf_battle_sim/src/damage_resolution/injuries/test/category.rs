//! GTW-440 — shared per-CATEGORY pool resolution (C1 / C5a) + side-from-struck-part
//! (C3 / C5b).

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

/// Build a single-entry catalog whose ONE injury key is authored into the ARM category
/// (its def's `category` is `Arm`) — so a roll keyed by EITHER arm must resolve this same
/// shared entry. The injury carries a `DisableHand` so the rolled verdict can be folded to
/// prove the side-from-part mapping.
fn shared_arm_table(key: &str) -> (InjuryRegistry, InjuryTables) {
    let name = InjuryName::new(key.to_owned());
    let def = InjuryDef {
        name:         name.clone(),
        // Authored into the shared Arm category (NOT a side); the rolled verdict's `part`
        // comes from the STRUCK side, not this field (GTW-440 C3 / GTW-453).
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
    // Insert keyed by the shared Arm category bucket, so a lookup from EITHER arm side
    // (`LeftArm` / `RightArm`) resolves the SAME table.
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
    // C5(a) — THE STRUCTURAL PIN: a roll on LeftArm AND a roll on RightArm BOTH resolve the
    // ONE shared Arm-category pool (the table was inserted keyed by LeftArm only). Each
    // rolls the catalog's single `shattered_hand`. Pin-discriminating: were the tables
    // still keyed per-SIDE (no category collapse), the RightArm roll would find an empty /
    // missing bucket and return None — this test would FAIL.
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
    // Both drew the same catalog entry (the one `shattered_hand` key).
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
    // C5(b) — THE SIDE-FROM-PART PIN: a `DisableHand` rolled on LeftArm disables the LEFT
    // hand, the SAME def rolled on RightArm disables the RIGHT hand — even though BOTH draw
    // from the ONE shared Arm pool whose def authors `category: Arm` (side-agnostic). The
    // rolled verdict's `part` is the STRUCK side, so folding it disables the correct hand.
    // Pin-discriminating: if the side were taken from the def's authored `category` (Arm,
    // which has no side) instead of the struck part, neither roll could distinguish left
    // from right and the two ledgers below would be identical — this test would FAIL.
    let (registry, tables) = shared_arm_table("shattered_hand");

    // Roll on the LEFT arm, fold the verdict, then roll on the RIGHT arm and fold it onto a
    // SEPARATE ledger. Both rolls draw the same shared-pool def.
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

    // The rolled verdict records the STRUCK side, not the def's `category`.
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

    // Fold ONLY the left-arm injury onto one ledger: the LEFT hand is disabled, the right
    // stays (so the right-hand-only weapon still has a hand). With one hand disabled, two
    // remain minus one = one hand available.
    let mut left_only = InflictedInjuries::default();
    left_only.gain(left_rolled.into_gained());
    assert_eq!(
        left_only.hands_available(),
        HandsAvailable::new(1),
        "a LeftArm DisableHand disables exactly one (the left) hand",
    );

    // Fold BOTH the left-arm and the right-arm injuries: BOTH hands disabled → zero. This
    // can ONLY reach zero if the two injuries disabled DIFFERENT sides (a set over sides);
    // were the side taken from the shared def (both LeftArm), the set would hold one side
    // and one hand would remain — failing this assertion.
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
    // C5(c): a MovementCostMul rolled on a LEG yields a Hampered movement factor > 1.0
    // (the ganger moves slower). Drives the real roll over a leg-category pool, folds the
    // verdict, and asserts the ledger's movement factor exceeds the identity 1.0.
    // Pin-discriminating: a factor of exactly 1.0 (identity) would mean the Hampered effect
    // never folded — this fails.
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

    // A RightLeg roll resolves the SAME shared Leg pool (inserted keyed by LeftLeg).
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
