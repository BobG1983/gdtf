//! Load injury contexts and prove authored weighting resolves.
use gdtf_battle_sim::{
    armor::{BodyPart, InjuryCategory},
    injuries::{DamageContext, InjuryTables},
    severity::Severity,
};
use gdtf_game::test_support::AppState;
use gdtf_test_utils::{GdtfLoadTestAppBuilder, advance_until_resource_exists};

const PART_PER_CATEGORY: [(InjuryCategory, BodyPart); 4] = [
    (InjuryCategory::Head, BodyPart::Head),
    (InjuryCategory::Torso, BodyPart::Torso),
    (InjuryCategory::Arm, BodyPart::LeftArm),
    (InjuryCategory::Leg, BodyPart::RightLeg),
];

#[test]
fn every_category_has_a_bucket_in_every_damage_context() {
    let mut app = GdtfLoadTestAppBuilder::new()
        .starting_in(AppState::Load)
        .build();
    advance_until_resource_exists::<InjuryTables>(&mut app);

    let Some(tables) = app.world().get_resource::<InjuryTables>() else {
        return;
    };
    for (category, part) in PART_PER_CATEGORY {
        for context in DamageContext::ALL {
            let any_bucket = [Severity::Minor, Severity::Major, Severity::Critical]
                .into_iter()
                .any(|severity| tables.table(part, context, severity).is_some());
            assert!(
                any_bucket,
                "the {category:?} pool must have at least one rollable bucket in the \
                 {context:?} context (the shipped weighting/{category:?}[.{context:?}] file \
                 must fold under its OWN context key)",
            );
        }
    }
}

#[test]
fn the_same_shared_pool_is_weighted_differently_per_source() {
    let mut app = GdtfLoadTestAppBuilder::new()
        .starting_in(AppState::Load)
        .build();
    advance_until_resource_exists::<InjuryTables>(&mut app);

    let Some(tables) = app.world().get_resource::<InjuryTables>() else {
        return;
    };
    let bucket = |context: DamageContext| tables.table(BodyPart::Head, context, Severity::Minor);
    let buckets = (
        bucket(DamageContext::Ranged),
        bucket(DamageContext::Melee),
        bucket(DamageContext::Fall),
    );
    assert!(
        buckets.0.is_some() && buckets.1.is_some() && buckets.2.is_some(),
        "the shipped head weighting files must populate the (Head, Minor) bucket in all \
         three contexts (ranged / melee / fall)",
    );
    let (Some(ranged), Some(melee), Some(fall)) = buckets else {
        return;
    };

    assert_ne!(
        ranged, melee,
        "the ranged and melee Head/Minor tables must DIFFER — the per-source weighting is the whole point",
    );
    assert!(
        ranged
            .iter()
            .any(|row| melee.iter().any(|other| other.injury == row.injury)),
        "the ranged and melee tables must weight at least one of the SAME shared injury \
         defs (re-weighting the one pool, never duplicating it)",
    );
    assert!(
        fall.iter()
            .any(|row| ranged.iter().any(|other| other.injury == row.injury)),
        "the fall table must likewise draw from the SAME shared Head pool",
    );
}
