//! The GTW-452 **damage-context selection** roll tests — the proof that the
//! [`DamageContext`] parameter threaded through [`roll_injury`] actually picks a DIFFERENT
//! per-source weighting table over the SAME shared per-category injury pool, and is not a
//! parameter that exists but is ignored.
//!
//! These exercise the REAL [`roll_injury`] path on the real types (no stubs): ONE shared
//! [`InjuryRegistry`] (the GTW-440 per-category pool — no duplicated def), and per-context
//! weighting tables built over that same pool. The RNG is a fixed-seed [`InjuryRng`], so the
//! picks are deterministic; the tests assert WHICH injury the context routes to, not any
//! tuned magnitude.

use super::{
    super::{
        DamageContext, InjuryDef, InjuryEffect, InjuryName, InjuryRegistry, InjuryTables,
        InjuryWeight, InspectText, LogText, PopupText, PostHeal, StatDelta, StatTarget,
        WeightedInjuryEntry, WeightedInjuryTable, roll_injury,
    },
    support::injury_rng,
};
use crate::{armor::BodyPart, severity::Severity};

/// The two shared-pool head injury keys the context tests weight against each other. Both
/// are authored ONCE into the single registry (below) — the per-context tables only re-weight
/// these SAME keys, never redefine them.
const BROKEN_NOSE: &str = "broken_nose";
const SCALP_GRAZE: &str = "scalp_graze";

/// Build a minimal head [`InjuryDef`] for `key` — a `Minor` head injury with one `Aim`
/// modify. The magnitude is irrelevant (the tests assert selection, not tuning).
fn head_def(key: &str) -> (InjuryName, InjuryDef) {
    let name = InjuryName::new(key.to_owned());
    let def = InjuryDef {
        name:         name.clone(),
        category:     BodyPart::Head.injury_category(),
        severity:     Severity::Minor,
        popup_text:   PopupText::new("HURT".to_owned()),
        log_text:     LogText::new("is hurt".to_owned()),
        inspect_text: InspectText::new("Hurt".to_owned()),
        effects:      vec![InjuryEffect::Modify {
            stat:   StatTarget::Aim,
            amount: StatDelta::new(-1),
        }],
        post_heal:    PostHeal::Deferred,
    };
    (name, def)
}

/// The ONE shared head injury pool — a single [`InjuryRegistry`] holding `broken_nose` and
/// `scalp_graze` each defined EXACTLY ONCE. Every per-context table below re-weights these
/// same two keys; no context owns a duplicate def (GTW-452 / GTW-440 C1).
fn shared_head_pool() -> InjuryRegistry {
    InjuryRegistry::new([head_def(BROKEN_NOSE), head_def(SCALP_GRAZE)])
}

/// One `Minor`-bucket head weighting row for `key` at `weight`.
fn row(key: &str, weight: u32) -> WeightedInjuryEntry {
    WeightedInjuryEntry::new(InjuryName::new(key.to_owned()), InjuryWeight::new(weight))
}

/// Insert a `(Head, context, Minor)` table built from `rows` into `tables`.
fn insert_head_minor(
    tables: &mut InjuryTables,
    context: DamageContext,
    rows: Vec<WeightedInjuryEntry>,
) {
    tables.insert(
        BodyPart::Head.injury_category(),
        context,
        Severity::Minor,
        WeightedInjuryTable::new(rows),
    );
}

/// AC4 (airtight): the context parameter is NOT ignored — with the SAME shared pool and the
/// SAME fixed-seed RNG, a `Ranged` roll and a `Melee` roll resolve DIFFERENT injuries, because
/// each context routes to its own single-entry per-source table over that one pool. If the
/// parameter were dropped, both rolls would hit the same table and pick the same injury.
#[test]
fn context_routes_to_a_different_per_source_table_over_the_same_shared_pool() {
    let registry = shared_head_pool();
    let mut tables = InjuryTables::default();
    // Ranged: only scalp_graze is weighted. Melee: only broken_nose is weighted. Same pool,
    // different per-source table — a bullet grazes the scalp, a fist breaks the nose.
    insert_head_minor(
        &mut tables,
        DamageContext::Ranged,
        vec![row(SCALP_GRAZE, 10)],
    );
    insert_head_minor(
        &mut tables,
        DamageContext::Melee,
        vec![row(BROKEN_NOSE, 10)],
    );

    let mut rng_ranged = injury_rng();
    let mut rng_melee = injury_rng();
    let ranged = roll_injury(
        BodyPart::Head,
        Severity::Minor,
        DamageContext::Ranged,
        &tables,
        &registry,
        &mut rng_ranged,
    );
    let melee = roll_injury(
        BodyPart::Head,
        Severity::Minor,
        DamageContext::Melee,
        &tables,
        &registry,
        &mut rng_melee,
    );

    assert!(
        ranged.is_some(),
        "the Ranged (Head, Minor) bucket is weighted → an injury rolls"
    );
    assert!(
        melee.is_some(),
        "the Melee (Head, Minor) bucket is weighted → an injury rolls"
    );
    let (Some(ranged), Some(melee)) = (ranged, melee) else {
        return;
    };
    assert_eq!(
        &**ranged.name, SCALP_GRAZE,
        "the ranged per-source table weights only scalp_graze"
    );
    assert_eq!(
        &**melee.name, BROKEN_NOSE,
        "the melee per-source table weights only broken_nose"
    );
    assert_ne!(
        ranged.name, melee.name,
        "the SAME seed under two contexts must pick two DIFFERENT injuries — proving the \
         context parameter routes the roll, not that it is accepted and ignored"
    );
}

/// AC2 (worked example, graded): the SAME two-injury pool appears in BOTH context tables, but
/// with FLIPPED weights — `broken_nose` high in melee / near-zero in ranged, `scalp_graze` the
/// reverse. Rolled across many seeds, the majority pick flips with the context, proving the
/// weight (not the pool membership) is what the context changes.
#[test]
fn flipped_weights_flip_the_majority_pick_over_the_shared_pool() {
    let registry = shared_head_pool();
    let mut tables = InjuryTables::default();
    // Both injuries present in both tables — only the weights differ per source.
    insert_head_minor(
        &mut tables,
        DamageContext::Ranged,
        vec![row(BROKEN_NOSE, 1), row(SCALP_GRAZE, 99)],
    );
    insert_head_minor(
        &mut tables,
        DamageContext::Melee,
        vec![row(BROKEN_NOSE, 99), row(SCALP_GRAZE, 1)],
    );

    let mut ranged_broken_nose = 0_u32;
    let mut melee_broken_nose = 0_u32;
    for seed in 0..200_u64 {
        let mut rng_r = super::support::injury_rng_from(seed);
        let mut rng_m = super::support::injury_rng_from(seed);
        if let Some(r) = roll_injury(
            BodyPart::Head,
            Severity::Minor,
            DamageContext::Ranged,
            &tables,
            &registry,
            &mut rng_r,
        ) && &**r.name == BROKEN_NOSE
        {
            ranged_broken_nose += 1;
        }
        if let Some(m) = roll_injury(
            BodyPart::Head,
            Severity::Minor,
            DamageContext::Melee,
            &tables,
            &registry,
            &mut rng_m,
        ) && &**m.name == BROKEN_NOSE
        {
            melee_broken_nose += 1;
        }
    }

    assert!(
        melee_broken_nose > 150,
        "melee weights broken_nose at 99% — it should dominate ({melee_broken_nose}/200)"
    );
    assert!(
        ranged_broken_nose < 50,
        "ranged weights broken_nose at 1% — it should be rare ({ranged_broken_nose}/200)"
    );
    assert!(
        melee_broken_nose > ranged_broken_nose,
        "the SAME broken_nose def is picked FAR more often under melee than ranged — the context \
         changes the weight over one shared pool, not the pool"
    );
}

/// AC3 (no duplication, sim level): every key referenced by every per-context table resolves in
/// the ONE shared [`InjuryRegistry`]. The context axis re-weights the pool; it never adds a def.
#[test]
fn every_context_table_key_resolves_in_the_single_shared_registry() {
    let registry = shared_head_pool();
    let mut tables = InjuryTables::default();
    insert_head_minor(
        &mut tables,
        DamageContext::Ranged,
        vec![row(BROKEN_NOSE, 1), row(SCALP_GRAZE, 99)],
    );
    insert_head_minor(
        &mut tables,
        DamageContext::Melee,
        vec![row(BROKEN_NOSE, 99), row(SCALP_GRAZE, 1)],
    );
    insert_head_minor(&mut tables, DamageContext::Fall, vec![row(SCALP_GRAZE, 10)]);

    for context in DamageContext::ALL {
        let Some(table) = tables.table(BodyPart::Head, context, Severity::Minor) else {
            continue;
        };
        for entry in table.iter() {
            assert!(
                registry.def(&entry.injury).is_some(),
                "context {context:?} weights `{}`, which must resolve in the ONE shared registry \
                 (no per-context duplicate def)",
                *entry.injury,
            );
        }
    }
}
