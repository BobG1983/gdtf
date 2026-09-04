//! file's authored `context:` field, parsed from REAL RON text through the real
//! fields → the `#[serde(default)]` ranged fallback) would silently REPLACE the ranged
use gdtf_battle_sim::{
    armor::{BodyPart, InjuryCategory},
    injuries::{DamageContext, InjuryName, InjuryTables},
    severity::Severity,
};

use super::support::{
    add_def, add_folder, add_weighting, app, build, injury_def, weighting, weighting_in_context,
};

fn three_context_tables() -> Option<InjuryTables> {
    let mut app = app();
    let graze = injury_def("Scalp Graze", InjuryCategory::Head, Severity::Minor)?;
    let nose = injury_def("Broken Nose", InjuryCategory::Head, Severity::Minor)?;
    // `#[serde(default)]` back-compat fallback, exercised through real RON text).
    let ranged = weighting(
        InjuryCategory::Head,
        &[("scalp_graze", 10), ("broken_nose", 1)],
        &[],
        &[],
    )?;
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

#[test]
fn authored_context_keys_its_own_bucket_over_the_shared_pool() {
    let Some(tables) = three_context_tables() else {
        return;
    };

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

    assert_eq!(
        weight_of(DamageContext::Ranged, &nose_key),
        Some(1),
        "a weighting file with NO `context:` field must build the Ranged table",
    );
    assert_eq!(
        weight_of(DamageContext::Melee, &nose_key),
        Some(12),
        "the `context: Melee` file must build the Melee table (the same shared def, \
         weighted differently)",
    );
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
