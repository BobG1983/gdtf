//! The per-source [`DamageContext`] axis of the table build (GTW-452) — a weighting
//! file's authored `context:` field, parsed from REAL RON text through the real
//! [`build_injury_data`](gdtf_content_families::injuries::build_injury_data), keys its
//! own bucket, and a context-LESS (pre-GTW-452) file falls back to the ranged table.
//!
//! These pin the loader half of GTW-452: the shipped
//! `weighting/<category>.<context>.weighting.ron` files only reach the roll if the
//! `context:` field survives deserialization AND the per-context key keeps the three
//! tables side by side. Without this, a misspelled `context:` key (serde ignores unknown
//! fields → the `#[serde(default)]` ranged fallback) would silently REPLACE the ranged
//! table on insert and the whole suite would stay green.

use gdtf_battle_sim::{
    armor::{BodyPart, InjuryCategory},
    injuries::{DamageContext, InjuryName, InjuryTables},
    severity::Severity,
};

use super::support::{
    add_def, add_folder, add_weighting, app, build, injury_def, weighting, weighting_in_context,
};

/// Stage the three per-source weighting files for ONE category over ONE shared pair of
/// injury defs and build them through the real loader — a context-LESS file (the
/// pre-GTW-452 shape), a `context: Melee` file, and a `context: Fall` file.
///
/// Returns `None` when a fixture could not be constructed, so the caller's assertions are
/// skipped rather than panicking (the no-`unwrap` rule); the fixture halves are split out
/// of the test body so each half stays readable on its own.
fn three_context_tables() -> Option<InjuryTables> {
    let mut app = app();
    // ONE shared pair of Head injury defs — every context table below weights THESE,
    // never a duplicated def (GTW-452 AC3).
    let graze = injury_def("Scalp Graze", InjuryCategory::Head, Severity::Minor)?;
    let nose = injury_def("Broken Nose", InjuryCategory::Head, Severity::Minor)?;
    // The pre-GTW-452 shape: NO `context:` field at all → the ranged table (the
    // `#[serde(default)]` back-compat fallback, exercised through real RON text).
    let ranged = weighting(
        InjuryCategory::Head,
        &[("scalp_graze", 10), ("broken_nose", 1)],
        &[],
        &[],
    )?;
    // The SAME two shared defs, weighted for melee (the broken nose leads) and for a fall.
    let melee = weighting_in_context(
        InjuryCategory::Head,
        DamageContext::Melee,
        &[("broken_nose", 12), ("scalp_graze", 4)],
        &[],
        &[],
    )?;
    let fall = weighting_in_context(
        InjuryCategory::Head,
        DamageContext::Fall,
        &[("scalp_graze", 6)],
        &[],
        &[],
    )?;

    let graze_handle = add_def(
        &mut app,
        "content/injuries/head/scalp_graze.injury.ron",
        graze,
    );
    let nose_handle = add_def(
        &mut app,
        "content/injuries/head/broken_nose.injury.ron",
        nose,
    );
    let ranged_handle = add_weighting(
        &mut app,
        "content/injuries/weighting/head.weighting.ron",
        ranged,
    );
    let melee_handle = add_weighting(
        &mut app,
        "content/injuries/weighting/head.melee.weighting.ron",
        melee,
    );
    let fall_handle = add_weighting(
        &mut app,
        "content/injuries/weighting/head.fall.weighting.ron",
        fall,
    );
    let folder = add_folder(
        &mut app,
        &[
            graze_handle.untyped(),
            nose_handle.untyped(),
            ranged_handle.untyped(),
            melee_handle.untyped(),
            fall_handle.untyped(),
        ],
    );

    let built = build(&app, &folder);
    assert!(
        built.is_some(),
        "build must succeed once every asset is in its collection",
    );
    built.map(|(_registry, tables)| tables)
}

/// The three per-source weighting files build THREE side-by-side buckets over the SAME
/// shared injury pool, each keyed by its own context, with the context-less file landing
/// in [`DamageContext::Ranged`].
///
/// Pin-discriminating: if the authored `context:` field were dropped on the parse (or the
/// tables key ignored it), all three files would fold into ONE bucket and the later insert
/// would replace the earlier — the melee / fall lookups below would then return the ranged
/// rows (or nothing) and this test goes red. It is the loader-side twin of the sim-side
/// `injuries::test::context` roll tests, and the only place a `context:` field is parsed
/// from RON TEXT.
#[test]
fn authored_context_keys_its_own_bucket_over_the_shared_pool() {
    let Some(tables) = three_context_tables() else {
        return;
    };

    // THREE buckets, one per context — no file replaced another.
    assert_eq!(
        tables.len(),
        3,
        "the three per-source weighting files must fold into three side-by-side buckets \
         (a lost `context:` would collapse them into one)",
    );

    let nose_key = InjuryName::new("broken_nose".to_owned());
    let graze_key = InjuryName::new("scalp_graze".to_owned());
    let weight_of = |context: DamageContext, key: &InjuryName| -> Option<u32> {
        tables
            .table(BodyPart::Head, context, Severity::Minor)?
            .iter()
            .find(|row| row.injury == *key)
            .map(|row| *row.weight)
    };

    // The context-LESS file landed in the RANGED bucket (the serde default), carrying its
    // own authored weights.
    assert_eq!(
        weight_of(DamageContext::Ranged, &nose_key),
        Some(1),
        "a weighting file with NO `context:` field must build the Ranged table",
    );
    // The `context: Melee` file built its OWN bucket over the SAME shared defs, with the
    // broken nose weighted up — the fixture's worked example of the GTW-452 rule.
    assert_eq!(
        weight_of(DamageContext::Melee, &nose_key),
        Some(12),
        "the `context: Melee` file must build the Melee table (the same shared def, \
         weighted differently)",
    );
    // The `context: Fall` file built the third bucket; it weights only the graze, so the
    // melee-only row must NOT leak into it.
    assert_eq!(
        weight_of(DamageContext::Fall, &graze_key),
        Some(6),
        "the `context: Fall` file must build the Fall table",
    );
    assert_eq!(
        weight_of(DamageContext::Fall, &nose_key),
        None,
        "a context's bucket must hold ONLY its own authored rows",
    );
}
