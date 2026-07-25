//! WARN-not-fail salvage paths — the unknown weighting key, the unweighted registry
//! injury, and the subfolder/category mismatch — each observed under `capture_logs`.

use gdtf_battle_sim::{
    armor::{BodyPart, InjuryCategory},
    injuries::{DamageContext, InjuryName},
    severity::Severity,
};

use super::support::{add_def, add_folder, add_weighting, app, build, injury_def, weighting};
use crate::states::load::systems::resolve::hot_reload_test_support::capture_logs;

/// C6: a weighting entry naming an UNKNOWN injury key is WARNED + SKIPPED — the build
/// still succeeds, the known entry survives, the unknown one is absent.
///
/// Pin-discriminating: a panic-on-unknown-key would fail the build; failing to skip it
/// would leave a dangling row.
#[test]
fn unknown_weighting_key_is_skipped_not_failed() {
    let mut app = app();
    let Some(eye) = injury_def("Lost Eye", InjuryCategory::Head, Severity::Critical) else {
        return;
    };
    // The weighting references the real `lost_eye` PLUS a `ghost` that has no def.
    let Some(w) = weighting(
        InjuryCategory::Head,
        &[],
        &[],
        &[("lost_eye", 4), ("ghost", 9)],
    ) else {
        return;
    };
    let def_h = add_def(&mut app, "content/injuries/head/lost_eye.injury.ron", eye);
    let w_h = add_weighting(&mut app, "content/injuries/weighting/head.weighting.ron", w);
    let folder = add_folder(&mut app, &[def_h.untyped(), w_h.untyped()]);

    // Build UNDER the log capture so the unknown-key skip WARN is observed too.
    let mut built = None;
    let captured = capture_logs(|| built = build(&app, &folder));
    assert!(
        built.is_some(),
        "an unknown weighting key must NOT fail the build"
    );
    let Some((_registry, tables)) = built else {
        return;
    };
    let bucket = tables.table(BodyPart::Head, DamageContext::Ranged, Severity::Critical);
    let keys: Vec<String> = bucket
        .map(|t| t.iter().map(|r| (*r.injury).clone()).collect())
        .unwrap_or_default();
    assert_eq!(
        keys,
        vec!["lost_eye".to_owned()],
        "only the known key survives; the unknown `ghost` row is skipped",
    );
    // WARN-emission pin: deleting the build_tables unknown-key `warn!` would empty this.
    assert!(
        captured
            .iter()
            .any(|line| { line.contains("unknown injury key") && line.contains("ghost") }),
        "the unknown weighting key must emit the skip WARN naming `ghost`; captured: {captured:?}",
    );
}

/// C6: a registered injury referenced by NO weighting bucket does NOT fail the build —
/// it is loaded into the registry and merely WARNED (it can never be rolled).
///
/// Pin-discriminating: treating the missing-weighting case as a failure would drop the
/// injury from the registry or return `None`; deleting the
/// [`audit_unweighted_injuries`](super::super::audit_unweighted_injuries) `warn!` empties
/// the WARN capture.
#[test]
fn registry_injury_with_no_weighting_does_not_fail() {
    let mut app = app();
    let Some(orphan) = injury_def("Orphan", InjuryCategory::Arm, Severity::Minor) else {
        return;
    };
    // A def with NO weighting file at all.
    let def_h = add_def(&mut app, "content/injuries/arm/orphan.injury.ron", orphan);
    let folder = add_folder(&mut app, &[def_h.untyped()]);

    // Build UNDER the log capture so the audit's unweighted-injury WARN is observed too.
    let mut built = None;
    let captured = capture_logs(|| built = build(&app, &folder));
    assert!(
        built.is_some(),
        "a registry injury with no weighting must NOT fail the build",
    );
    let Some((registry, tables)) = built else {
        return;
    };
    let key = InjuryName::new("orphan.injury".to_owned());
    // Stem strip yields `orphan` (the `.injury` infix removed).
    let resolved = InjuryName::new("orphan".to_owned());
    assert!(
        registry.contains(&resolved) && !registry.contains(&key),
        "the orphan injury is registered by its stripped stem key",
    );
    assert!(tables.is_empty(), "no weighting => no table built");
    // WARN-emission pin: deleting the audit_unweighted_injuries `warn!` would empty this.
    assert!(
        captured
            .iter()
            .any(|line| { line.contains("is in no weighting table") && line.contains("orphan") }),
        "the unweighted registry injury must emit the audit WARN; captured: {captured:?}",
    );
}

/// C4 (mismatch clause): a def whose authored `category` does NOT match its owning
/// per-category subfolder is loaded under ITS OWN field (the def is authoritative, the
/// subfolder is organizational, design fork #10) AND a WARN is emitted — proving the
/// `subfolder != def.category` branch in `warn_on_subfolder_mismatch` is traversed.
///
/// Pin-discriminating: neutralizing the mismatch branch (so the warn never fires) empties
/// the WARN capture; mis-handling the authoritative field (filing under the subfolder
/// `Torso`) fails the exact-equality `category` + bucket assertions.
#[test]
fn subfolder_mismatch_warns_but_loads_authoritative_category() {
    let mut app = app();
    // A Head-category injury authored into the `torso/` subfolder — category != subfolder.
    let Some(misfiled) = injury_def("Misfiled", InjuryCategory::Head, Severity::Major) else {
        return;
    };
    // A weighting for the def's AUTHORITATIVE category (Head) so it lands in a table; were
    // the def mis-filed under Torso, this row would resolve to no bucket.
    let Some(w) = weighting(InjuryCategory::Head, &[], &[("misfiled", 5)], &[]) else {
        return;
    };
    let def_h = add_def(
        &mut app,
        "content/injuries/torso/misfiled.injury.ron",
        misfiled,
    );
    let w_h = add_weighting(&mut app, "content/injuries/weighting/head.weighting.ron", w);
    let folder = add_folder(&mut app, &[def_h.untyped(), w_h.untyped()]);

    // Build UNDER the log capture so the mismatch WARN is observed.
    let mut built = None;
    let captured = capture_logs(|| built = build(&app, &folder));
    assert!(
        built.is_some(),
        "a subfolder mismatch must NOT fail the build"
    );
    let Some((registry, tables)) = built else {
        return;
    };

    // (a) AUTHORITATIVE-FIELD-WINS: the injury is registered, and its registered
    // category is Head (its own field), NEVER Torso (the subfolder). Exact equality.
    let key = InjuryName::new("misfiled".to_owned());
    let def = registry.def(&key);
    assert!(
        def.is_some(),
        "the misfiled injury must still be registered"
    );
    assert_eq!(
        def.map(|d| d.category),
        Some(InjuryCategory::Head),
        "the registered category must be the def's own Head, NOT the Torso subfolder",
    );
    // It tables into the (Head, Major) bucket, NEVER the (Torso, Major) bucket.
    let head_bucket = tables.table(BodyPart::Head, DamageContext::Ranged, Severity::Major);
    assert!(
        head_bucket.is_some_and(|t| t.iter().any(|r| r.injury == key)),
        "the misfiled injury must table under (Head, Major), its authoritative part",
    );
    assert!(
        tables
            .table(BodyPart::Torso, DamageContext::Ranged, Severity::Major)
            .is_none(),
        "nothing must land in the (Torso, Major) bucket — the subfolder is not authoritative",
    );

    // (b) WARN-EMITS: the warn_on_subfolder_mismatch WARN fired on the mismatch.
    assert!(
        captured.iter().any(|line| {
            line.contains("subfolder") && line.contains("Head") && line.contains("Torso")
        }),
        "the subfolder mismatch must emit the WARN naming both parts; captured: {captured:?}",
    );
}
