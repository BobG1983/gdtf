//! GTW-437 loader tests — the injury registry + tables build, the WARN-not-fail
//! handling, the canonical-sort determinism, and the headless hot-reload rebuild.
//!
//! These drive the REAL loader code path: the headless hot-reload test runs the actual
//! [`redrive_injuries_on_asset_event`] system inside a `MinimalPlugins` + `AssetPlugin`
//! app (the weapons-test harness, generalised to two asset types), and the build /
//! determinism / WARN tests exercise the actual private [`build_injury_data`] /
//! `build_tables` against in-memory assets. Per the loader-tests rule they assert
//! STRUCTURE (parse-OK / key-resolves / determinism / WARN-not-fail), never specific
//! shipped tunable magnitudes.

use bevy::{
    MinimalPlugins,
    asset::{AssetEvent, AssetPlugin, AssetServer, Assets, Handle, LoadedFolder},
    ecs::system::RunSystemOnce,
    prelude::*,
};
use gdtf_assets::{RonAsset, RonAssetAppExt};
use gdtf_battle_sim::{
    armor::BodyPart,
    injuries::{InjuryDef, InjuryName, InjuryRegistry, InjuryTables, InjuryWeighting},
    rng::{BattleSeed, InjuryRng},
    severity::Severity,
};

use super::{build_injury_data, redrive_injuries_on_asset_event};
use crate::states::load::{
    resources::ActiveInjuriesFolderHandle, systems::resolve::hot_reload_test_support::capture_logs,
};

/// Parse a sample-shaped `InjuryDef` from inline RON (the schema the loader reads),
/// asserting it parses rather than a denied `unwrap`.
fn injury_def(name: &str, part: BodyPart, severity: Severity) -> Option<InjuryDef> {
    let ron = format!(
        "(name: \"{name}\", body_part: {part:?}, severity: {severity:?}, \
         popup_text: \"{name}\", log_text: \"is hurt\", inspect_text: \"{name} -- -1 Aim\", \
         effects: [Modify(stat: Aim, amount: -1)])",
    );
    let parsed = ron::de::from_str::<InjuryDef>(&ron);
    assert!(
        parsed.is_ok(),
        "injury fixture must parse: {:?}",
        parsed.as_ref().err()
    );
    parsed.ok()
}

/// Parse a sample-shaped `InjuryWeighting` from inline RON, asserting it parses.
/// `minor` / `major` / `critical` are `(key, weight)` row lists.
fn weighting(
    part: BodyPart,
    minor: &[(&str, u32)],
    major: &[(&str, u32)],
    critical: &[(&str, u32)],
) -> Option<InjuryWeighting> {
    let rows = |list: &[(&str, u32)]| {
        list.iter()
            .map(|(k, w)| format!("(injury: \"{k}\", weight: {w})"))
            .collect::<Vec<_>>()
            .join(", ")
    };
    let ron = format!(
        "(body_part: {part:?}, minor: [{}], major: [{}], critical: [{}])",
        rows(minor),
        rows(major),
        rows(critical),
    );
    let parsed = ron::de::from_str::<InjuryWeighting>(&ron);
    assert!(
        parsed.is_ok(),
        "weighting fixture must parse: {:?}",
        parsed.as_ref().err()
    );
    parsed.ok()
}

/// A headless app with the real injury hot-reload wiring: `MinimalPlugins` +
/// `AssetPlugin` (registers `Assets<RonAsset<InjuryDef>>`, `Assets<RonAsset<InjuryWeighting>>`,
/// `Assets<LoadedFolder>`, and the `AssetEvent` buffers), BOTH injury RON loaders, and
/// the redrive system in `Update`.
fn app() -> App {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins)
        .add_plugins(AssetPlugin::default())
        .init_ron_asset_with_extensions::<InjuryDef>(vec!["injury.ron"])
        .init_ron_asset_with_extensions::<InjuryWeighting>(vec!["weighting.ron"])
        .add_systems(Update, redrive_injuries_on_asset_event);
    app
}

/// Register a member injury-def asset at `path` carrying `def`, returning its handle.
fn add_def(app: &mut App, path: &'static str, def: InjuryDef) -> Handle<RonAsset<InjuryDef>> {
    let handle = app
        .world()
        .resource::<AssetServer>()
        .load::<RonAsset<InjuryDef>>(path);
    let inserted = app
        .world_mut()
        .resource_mut::<Assets<RonAsset<InjuryDef>>>()
        .insert(handle.id(), RonAsset::new(def));
    assert!(inserted.is_ok(), "member def insert must succeed");
    handle
}

/// Register a member weighting asset at `path` carrying `weighting`, returning its handle.
fn add_weighting(
    app: &mut App,
    path: &'static str,
    weighting: InjuryWeighting,
) -> Handle<RonAsset<InjuryWeighting>> {
    let handle = app
        .world()
        .resource::<AssetServer>()
        .load::<RonAsset<InjuryWeighting>>(path);
    let inserted = app
        .world_mut()
        .resource_mut::<Assets<RonAsset<InjuryWeighting>>>()
        .insert(handle.id(), RonAsset::new(weighting));
    assert!(inserted.is_ok(), "member weighting insert must succeed");
    handle
}

/// Build a `LoadedFolder` over the given member handles (defs + weightings, untyped),
/// add it, return its handle.
fn add_folder(app: &mut App, members: &[bevy::asset::UntypedHandle]) -> Handle<LoadedFolder> {
    let folder = LoadedFolder {
        handles: members.to_vec(),
    };
    app.world_mut()
        .resource_mut::<Assets<LoadedFolder>>()
        .add(folder)
}

/// Run the real [`build_injury_data`] over the app's current assets + folder, returning
/// the built `(InjuryRegistry, InjuryTables)`.
fn build(app: &App, folder: &Handle<LoadedFolder>) -> Option<(InjuryRegistry, InjuryTables)> {
    let world = app.world();
    build_injury_data(
        world.resource::<AssetServer>(),
        world.resource::<Assets<LoadedFolder>>(),
        world.resource::<Assets<RonAsset<InjuryDef>>>(),
        world.resource::<Assets<RonAsset<InjuryWeighting>>>(),
        folder,
    )
}

/// C2/C4: the sample-shaped RON parses, the injury keys resolve from their file stems,
/// and the weighting folds into a `(part, severity)` table — the happy path.
///
/// Pin-discriminating: a broken stem-key strip drops the `lost_eye` entry; a broken
/// extension partition mis-files the weighting as an injury (or vice-versa).
#[test]
fn builds_registry_and_tables_keyed_by_stem() {
    let mut app = app();
    let Some(eye) = injury_def("Lost Eye", BodyPart::Head, Severity::Critical) else {
        return;
    };
    let Some(w) = weighting(BodyPart::Head, &[], &[], &[("lost_eye", 4)]) else {
        return;
    };
    let def_handle = add_def(&mut app, "content/injuries/head/lost_eye.injury.ron", eye);
    let w_handle = add_weighting(&mut app, "content/injuries/weighting/head.weighting.ron", w);
    let folder = add_folder(&mut app, &[def_handle.untyped(), w_handle.untyped()]);

    let built = build(&app, &folder);
    assert!(
        built.is_some(),
        "build must succeed once both assets are in their collections",
    );
    let Some((registry, tables)) = built else {
        return;
    };

    // Key resolves from the `.injury.ron` stem.
    let key = InjuryName::new("lost_eye".to_owned());
    assert!(registry.contains(&key), "lost_eye must resolve by stem");
    assert_eq!(registry.len(), 1, "exactly one injury registered");
    // The weighting folded into the Critical/Head bucket (only tabled buckets exist).
    let bucket = tables.table(BodyPart::Head, Severity::Critical);
    assert!(
        bucket.is_some_and(|t| t.iter().any(|r| r.injury == key)),
        "the Critical/Head bucket must hold lost_eye",
    );
    // No empty bucket is published (the empty Minor/Major lists table nothing).
    assert_eq!(tables.len(), 1, "only the non-empty bucket is tabled");
}

/// C4/determinism: the built bucket is CANONICALLY SORTED by injury key, so the same
/// rows authored in EITHER order produce the IDENTICAL table — and a seeded
/// cumulative-weight pick over that table is therefore enumeration-order-independent
/// (same seed → same pick). The roll itself is GTW-438; this exercises the table build
/// + a LOCAL seeded pick over the canonical-sorted rows.
///
/// Pin-discriminating: dropping the canonical sort makes the two tables differ and the
/// two picks diverge for an order-sensitive seed.
#[test]
fn canonical_sort_makes_the_seeded_pick_order_independent() {
    let pick = |entries_first: bool| -> Option<InjuryName> {
        let mut app = app();
        // Two injuries in the same bucket, authored in OPPOSITE orders across the runs.
        let a = injury_def("Alpha", BodyPart::Torso, Severity::Major)?;
        let b = injury_def("Bravo", BodyPart::Torso, Severity::Major)?;
        let a_h = add_def(&mut app, "content/injuries/torso/alpha.injury.ron", a);
        let b_h = add_def(&mut app, "content/injuries/torso/bravo.injury.ron", b);
        // The weighting lists the rows in opposite orders depending on the flag.
        let rows: &[(&str, u32)] = if entries_first {
            &[("alpha", 3), ("bravo", 7)]
        } else {
            &[("bravo", 7), ("alpha", 3)]
        };
        let w = weighting(BodyPart::Torso, &[], rows, &[])?;
        let w_h = add_weighting(
            &mut app,
            "content/injuries/weighting/torso.weighting.ron",
            w,
        );
        let folder = add_folder(&mut app, &[a_h.untyped(), b_h.untyped(), w_h.untyped()]);
        let (_registry, tables) = build(&app, &folder)?;
        let table = tables.table(BodyPart::Torso, Severity::Major)?;

        // A LOCAL seeded cumulative-weight pick over the canonical-sorted rows (the
        // GTW-438 roll's shape, kept local so this test owns no production pick).
        let total: u32 = table.iter().map(|r| *r.weight).sum();
        let mut rng = InjuryRng::from_root(BattleSeed::new(0xDEAD_BEEF));
        let draw: u32 = rng.random_range(0..total);
        let mut cumulative = 0u32;
        table.iter().find_map(|row| {
            cumulative += *row.weight;
            (draw < cumulative).then(|| row.injury.clone())
        })
    };

    let one = pick(true);
    let two = pick(false);
    assert!(one.is_some(), "the pick must resolve to an injury");
    assert_eq!(
        one, two,
        "the canonical sort must make the seeded pick identical regardless of authored \
         (folder-enumeration) order",
    );
}

/// C6: a weighting entry naming an UNKNOWN injury key is WARNED + SKIPPED — the build
/// still succeeds, the known entry survives, the unknown one is absent.
///
/// Pin-discriminating: a panic-on-unknown-key would fail the build; failing to skip it
/// would leave a dangling row.
#[test]
fn unknown_weighting_key_is_skipped_not_failed() {
    let mut app = app();
    let Some(eye) = injury_def("Lost Eye", BodyPart::Head, Severity::Critical) else {
        return;
    };
    // The weighting references the real `lost_eye` PLUS a `ghost` that has no def.
    let Some(w) = weighting(BodyPart::Head, &[], &[], &[("lost_eye", 4), ("ghost", 9)]) else {
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
    let bucket = tables.table(BodyPart::Head, Severity::Critical);
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
/// [`audit_unweighted_injuries`](super::audit_unweighted_injuries) `warn!` empties the
/// WARN capture.
#[test]
fn registry_injury_with_no_weighting_does_not_fail() {
    let mut app = app();
    let Some(orphan) = injury_def("Orphan", BodyPart::LeftArm, Severity::Minor) else {
        return;
    };
    // A def with NO weighting file at all.
    let def_h = add_def(
        &mut app,
        "content/injuries/left_arm/orphan.injury.ron",
        orphan,
    );
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

/// C4 (mismatch clause): a def whose authoritative `body_part` does NOT match its owning
/// per-part subfolder is loaded under ITS OWN field (the def is authoritative, the
/// subfolder is organizational, design fork #10) AND a WARN is emitted — proving the
/// `subfolder != def.body_part` branch in `warn_on_subfolder_mismatch` is traversed.
///
/// Pin-discriminating: neutralizing the mismatch branch (so the warn never fires) empties
/// the WARN capture; mis-handling the authoritative field (filing under the subfolder
/// `Torso`) fails the exact-equality `body_part` + bucket assertions.
#[test]
fn subfolder_mismatch_warns_but_loads_authoritative_body_part() {
    let mut app = app();
    // A Head injury authored into the `torso/` subfolder — body_part != subfolder.
    let Some(misfiled) = injury_def("Misfiled", BodyPart::Head, Severity::Major) else {
        return;
    };
    // A weighting for the def's AUTHORITATIVE part (Head) so it lands in a table; were the
    // def mis-filed under Torso, this row would resolve to no bucket.
    let Some(w) = weighting(BodyPart::Head, &[], &[("misfiled", 5)], &[]) else {
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
    // body_part is Head (its own field), NEVER Torso (the subfolder). Exact equality.
    let key = InjuryName::new("misfiled".to_owned());
    let def = registry.def(&key);
    assert!(
        def.is_some(),
        "the misfiled injury must still be registered"
    );
    assert_eq!(
        def.map(|d| d.body_part),
        Some(BodyPart::Head),
        "the registered body_part must be the def's own Head, NOT the Torso subfolder",
    );
    // It tables into the (Head, Major) bucket, NEVER the (Torso, Major) bucket.
    let head_bucket = tables.table(BodyPart::Head, Severity::Major);
    assert!(
        head_bucket.is_some_and(|t| t.iter().any(|r| r.injury == key)),
        "the misfiled injury must table under (Head, Major), its authoritative part",
    );
    assert!(
        tables.table(BodyPart::Torso, Severity::Major).is_none(),
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

/// C5: a `Modified` for a member `*.injury.ron` OR `*.weighting.ron` REBUILDS BOTH the
/// `InjuryRegistry` and the `InjuryTables` from the folder's members, reflecting the
/// edit — the headless hot-reload path (the weapons mirror, two resources).
///
/// Pin-discriminating: dropping the rebuild leaves the OLD (empty) resources.
#[test]
fn modified_member_rebuilds_both_resources() {
    let mut app = app();
    let Some(eye) = injury_def("Lost Eye", BodyPart::Head, Severity::Critical) else {
        return;
    };
    let Some(w) = weighting(BodyPart::Head, &[], &[], &[("lost_eye", 4)]) else {
        return;
    };
    let def_h = add_def(&mut app, "content/injuries/head/lost_eye.injury.ron", eye);
    let w_h = add_weighting(&mut app, "content/injuries/weighting/head.weighting.ron", w);
    let w_id = w_h.id();
    let folder = add_folder(&mut app, &[def_h.untyped(), w_h.untyped()]);
    app.world_mut()
        .insert_resource(ActiveInjuriesFolderHandle::new(folder));
    // Stale baseline (empty) resources the rebuild must overwrite.
    app.world_mut().insert_resource(InjuryRegistry::default());
    app.world_mut().insert_resource(InjuryTables::default());

    // Fire a Modified for the weighting member, rebuild.
    app.world_mut()
        .write_message(AssetEvent::Modified { id: w_id });
    app.update();

    let key = InjuryName::new("lost_eye".to_owned());
    let registry_has = app
        .world()
        .get_resource::<InjuryRegistry>()
        .is_some_and(|r| r.contains(&key));
    let tables_len = app
        .world()
        .get_resource::<InjuryTables>()
        .map(InjuryTables::len);
    assert!(
        registry_has,
        "the rebuild must repopulate the InjuryRegistry"
    );
    assert_eq!(
        tables_len,
        Some(1),
        "the rebuild must repopulate the InjuryTables (the Critical/Head bucket)",
    );
}

/// C5: a hot-reload of an injury member fires the Part C `info!` line naming what
/// reloaded. Run via `run_system_once` on the calling thread so the thread-local
/// `tracing` capture sees the emission.
///
/// Pin-discriminating: removing the `info!` leaves the capture empty.
#[test]
fn injury_hot_reload_logs_an_info_line() {
    let mut app = app();
    let Some(eye) = injury_def("Lost Eye", BodyPart::Head, Severity::Critical) else {
        return;
    };
    let def_h = add_def(&mut app, "content/injuries/head/lost_eye.injury.ron", eye);
    let def_id = def_h.id();
    let folder = add_folder(&mut app, &[def_h.untyped()]);
    app.world_mut()
        .insert_resource(ActiveInjuriesFolderHandle::new(folder));
    app.world_mut().insert_resource(InjuryRegistry::default());
    app.world_mut().insert_resource(InjuryTables::default());
    app.world_mut()
        .write_message(AssetEvent::Modified { id: def_id });

    let captured = capture_logs(|| {
        let result = app
            .world_mut()
            .run_system_once(redrive_injuries_on_asset_event);
        assert!(result.is_ok(), "the redrive system must run cleanly");
    });

    assert!(
        captured.iter().any(|line| {
            line.contains("injury hot-reload")
                && line.contains("InjuryRegistry")
                && line.contains("InjuryTables")
        }),
        "the injury hot-reload must emit an info! line naming both resources; captured: {captured:?}",
    );
}
