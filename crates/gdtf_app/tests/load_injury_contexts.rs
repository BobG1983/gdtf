//! GTW-452 real-asset coverage for the PER-SOURCE weighting files: with a real
//! `AssetServer` rooted at the workspace `assets/`, the shipped
//! `assets/content/injuries/weighting/<category>[.<context>].weighting.ron` set folds
//! into the [`InjuryTables`] under THREE distinct [`DamageContext`] keys — ranged,
//! melee, and fall — over the ONE shared per-category injury pool.
//!
//! This is the shipped-content twin of the sim-side roll tests
//! (`gdtf_battle_sim::injuries::test::context`, which prove the context parameter changes
//! the pick) and of the loader unit test
//! (`states::load::systems::resolve::injuries::test::context`, which proves the authored
//! `context:` field parses from RON text). What only THIS test can catch: a shipped melee
//! / fall file whose `context:` line is missing or misspelled. Serde ignores unknown
//! fields, so such a file deserializes as a SECOND ranged table and REPLACES the ranged
//! one on insert — silently emptying that context in live play (the fall path keys on
//! [`DamageContext::Fall`] today) with no other test going red.
//!
//! VALUE-AGNOSTIC per the brittle-test rule: it asserts bucket PRESENCE per
//! `(category, context)` and that the ranged and melee tables of one bucket DIFFER while
//! sharing an injury key — never an authored weight, never a bucket count. A tuning edit
//! to any weight cannot redden it.

use gdtf_app::test_support::AppState;
use gdtf_battle_sim::{
    armor::{BodyPart, InjuryCategory},
    injuries::{DamageContext, InjuryTables},
    severity::Severity,
};
use gdtf_test_utils::{GdtfLoadTestAppBuilder, advance_until_resource_exists};

/// Generous SAFETY-NET cap for the real-asset wait gated on the async injuries folder
/// load resolving (the `load_injuries.rs` precedent — a signal poll, not a timing budget).
const LOAD_SAFETY_NET: u32 = 10_000;

/// One body part per injury-pool category — the lookup side of the shared pool (a
/// per-side part resolves to its category at the table boundary).
const PART_PER_CATEGORY: [(InjuryCategory, BodyPart); 4] = [
    (InjuryCategory::Head, BodyPart::Head),
    (InjuryCategory::Torso, BodyPart::Torso),
    (InjuryCategory::Arm, BodyPart::LeftArm),
    (InjuryCategory::Leg, BodyPart::RightLeg),
];

/// AC1/AC2 (shipped content) — every injury-pool category has at least one rollable
/// bucket in EVERY [`DamageContext`], so the melee and fall weighting files the ticket
/// ships actually reach the built tables.
///
/// PIN: drop or misspell the `context:` line in any shipped `*.melee.weighting.ron` /
/// `*.fall.weighting.ron` and that file folds into the ranged key instead — its context
/// loses every bucket for that category and this test goes red.
#[test]
fn every_category_has_a_bucket_in_every_damage_context() {
    let mut app = GdtfLoadTestAppBuilder::new()
        .starting_in(AppState::Load)
        .build();
    advance_until_resource_exists::<InjuryTables>(&mut app, LOAD_SAFETY_NET);

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

/// AC2 (the worked example, over shipped content) — the SAME shared injury pool is
/// weighted DIFFERENTLY per source: the Head `Minor` bucket exists in all three contexts,
/// the ranged and melee tables are NOT the same table, and they share at least one injury
/// KEY (so the difference is weighting, not a duplicated / disjoint set of defs).
///
/// PIN: collapse the context axis (one table for every source) and the ranged / melee
/// tables become identical — red. Duplicate the pool per context instead of re-weighting
/// it, and the shared-key assertion goes red.
#[test]
fn the_same_shared_pool_is_weighted_differently_per_source() {
    let mut app = GdtfLoadTestAppBuilder::new()
        .starting_in(AppState::Load)
        .build();
    advance_until_resource_exists::<InjuryTables>(&mut app, LOAD_SAFETY_NET);

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
        "the ranged and melee Head/Minor tables must DIFFER — the per-source weighting is \
         the whole point (GTW-452)",
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
