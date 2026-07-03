//! GTW-437: builds the [`InjuryRegistry`] (name→def) AND the [`InjuryTables`]
//! (per-`(category, severity)` weighted table) from the loaded `assets/content/injuries/`
//! folder, plus the GTW-374 LIVE hot-reload that rebuilds BOTH on an edit to ANY
//! member `*.injury.ron` OR `*.weighting.ron`.
//!
//! One recursive [`LoadedFolder`] over `assets/content/injuries/` carries TWO asset types,
//! discriminated by their compound extension infix: the per-injury `*.injury.ron`
//! files (each a `RonAsset<InjuryDef>`, scattered across the four per-CATEGORY subfolders
//! `head` / `torso` / `arm` / `leg`, GTW-440) and the per-category
//! `weighting/*.weighting.ron` files (each a
//! `RonAsset<InjuryWeighting>`). The shared [`build_injury_data`] partitions the
//! folder's member handles by extension, keys the injuries by file stem into the
//! [`InjuryRegistry`], and folds the weighting files into the [`InjuryTables`] with
//! CANONICALLY-SORTED rows so folder-enumeration order can never change the
//! cumulative-weight pick (determinism). This mirrors the GTW-257 weapons resolve
//! shape (`super::weapons`), generalised to one folder → two resources.

use bevy::{
    asset::{AssetEvent, AssetServer, Assets, LoadedFolder, RecursiveDependencyLoadState},
    prelude::{Commands, MessageReader, Res, ResMut, info, warn},
};
use gdtf_assets::RonAsset;
use gdtf_battle_sim::{
    injuries::{
        InjuryDef, InjuryName, InjuryRegistry, InjuryTables, InjuryWeighting, WeightedInjuryEntry,
        WeightedInjuryTable,
    },
    severity::Severity,
};

use crate::states::load::resources::{ActiveInjuriesFolderHandle, LoadHandles};

/// GTW-437: builds the [`InjuryRegistry`] + [`InjuryTables`] from the loaded
/// `assets/content/injuries/` folder — the GTW-257 weapons resolve shape (that family
/// now rides the GTW-570 generic content-family seam) — one folder, two resources.
///
/// Called only while no [`InjuryRegistry`] resource exists yet (the caller's
/// own-absence guard), independently of the other resolve branches:
///
/// - Gates on the injuries folder's
///   [`RecursiveDependencyLoadState`]`::Loaded` (recursive, so every injury / weighting
///   `.ron` in the per-part + `weighting/` subfolders is loaded). On
///   [`RecursiveDependencyLoadState::Failed`] it `warn!`s and inserts an EMPTY
///   [`InjuryRegistry`] + [`InjuryTables`] so `Load` always exits with both present and
///   never hangs on a bad folder (the ADR-0003 error-path safety-net; the roll then
///   fails closed — no injury rolled — rather than crashing).
/// - On success it builds BOTH resources from the folder's member handles via the
///   shared [`build_injury_data`]. If ANY member asset is not yet in its collection
///   (the one-frame loaded-but-not-yet-in-collection race), it returns WITHOUT
///   inserting and retries next frame — so partial / empty resources are never
///   published while the folder is non-empty. The resources hold their data BY VALUE,
///   so they survive the folder handle being dropped on `OnExit(Load)`.
/// - Else (still loading) it does nothing and is polled again next frame.
pub(super) fn resolve_injuries(
    commands: &mut Commands,
    asset_server: &AssetServer,
    folders: &Assets<LoadedFolder>,
    injury_defs: &Assets<RonAsset<InjuryDef>>,
    weightings: &Assets<RonAsset<InjuryWeighting>>,
    handles: &LoadHandles,
) {
    let folder_state = asset_server.recursive_dependency_load_state(&*handles.injuries);

    // Failure path: a bad/missing injuries folder must not hang the app. Warn and
    // insert EMPTY resources so Load always exits with both present (the roll then
    // fails closed — no injury rolled — rather than crashing).
    if matches!(folder_state, RecursiveDependencyLoadState::Failed(_)) {
        warn!(
            "GDTF Load: the `injuries` folder failed to load; inserting an empty \
             InjuryRegistry + InjuryTables (the injury roll will find no table and \
             inflict no injury)",
        );
        commands.insert_resource(InjuryRegistry::default());
        commands.insert_resource(InjuryTables::default());
        return;
    }

    // Success path: once every injury / weighting file in the folder is loaded, build
    // both resources from the folder's member handles.
    if matches!(folder_state, RecursiveDependencyLoadState::Loaded) {
        let Some((registry, tables)) = build_injury_data(
            asset_server,
            folders,
            injury_defs,
            weightings,
            &handles.injuries,
        ) else {
            // Loaded-but-not-yet-in-collection (the folder, or a member asset) — retry
            // next frame (the system stays alive while the InjuryRegistry is absent).
            return;
        };

        // Insert the built resources — like the WeaponRegistry they persist past
        // OnExit(Load) (they are NOT removed in cleanup), because the roll reads them.
        commands.insert_resource(registry);
        commands.insert_resource(tables);
        // GTW-374: insert the PERSISTENT folder handle alongside the resources — it
        // survives OnExit(Load) so the live hot-reload handler can re-enumerate the
        // folder's member handles to rebuild both resources on an edit, and holding it
        // keeps every member injury / weighting asset loaded for the file-watcher.
        commands.insert_resource(ActiveInjuriesFolderHandle::new((*handles.injuries).clone()));
    }
}

/// Build BOTH the [`InjuryRegistry`] and the [`InjuryTables`] from a loaded
/// `injuries/` [`LoadedFolder`], or [`None`] if the folder (or any member asset) is
/// not yet in its collection — the injuries analogue of `build_weapon_registry`.
///
/// Shared by [`resolve_injuries`] (the one-time `Load`-state build) and the GTW-374
/// [`redrive_injuries_on_asset_event`] (the live rebuild on a hot edit), so both build
/// the resources IDENTICALLY:
///
/// - **Partition** the folder's member handles by their compound extension infix: a
///   path ending `.injury.ron` is a `RonAsset<InjuryDef>`, a path ending
///   `.weighting.ron` is a `RonAsset<InjuryWeighting>`. A member with neither infix is
///   skipped defensively.
/// - **Key** each injury by its file STEM minus the `.injury` infix (so
///   `head/lost_eye.injury.ron` keys `lost_eye`), into the [`InjuryRegistry`].
/// - **Validate** each def's authored [`category`](InjuryDef::category) against the owning
///   per-category subfolder name (`injuries/<category>/…`, GTW-440 / GTW-453): a
///   cross-category mismatch only `warn!`s — the
///   injury is STILL loaded using its own `category` field (the def is authoritative,
///   the subfolder is organizational, design fork #10).
/// - **Fold** every weighting file's three severity lists into the [`InjuryTables`],
///   resolving each row's key against the registry: an UNKNOWN key `warn!`s + is
///   skipped (never fails), and the surviving rows are CANONICALLY SORTED (by injury
///   key then weight) so folder-enumeration order can never change the cumulative-weight
///   pick.
/// - **Audit** the missing-weighting rule: any registered injury referenced by NO
///   weighting bucket `warn!`s (it can never be rolled) — a warning, never a failure.
///
/// Returns [`None`] (do NOT publish partial resources) if the folder or any member
/// asset is not yet in its collection — the caller retries next frame.
fn build_injury_data(
    asset_server: &AssetServer,
    folders: &Assets<LoadedFolder>,
    injury_defs: &Assets<RonAsset<InjuryDef>>,
    weightings: &Assets<RonAsset<InjuryWeighting>>,
    folder_handle: &bevy::asset::Handle<LoadedFolder>,
) -> Option<(InjuryRegistry, InjuryTables)> {
    let folder = folders.get(folder_handle)?;

    let mut registry = InjuryRegistry::default();
    let mut authored_weightings: Vec<InjuryWeighting> = Vec::new();

    for untyped in &folder.handles {
        // Resolve the member's asset path so we can both classify it (by extension
        // infix) and key it (by stem). A handle with no resolvable path is skipped
        // defensively (it carries no usable key / type).
        let Some(asset_path) = asset_server.get_path(untyped.id()) else {
            continue;
        };
        let path = asset_path.path();
        let path_str = path.to_string_lossy();

        if path_str.ends_with(".injury.ron") {
            // A per-injury `RonAsset<InjuryDef>`. One-frame race: not yet in the
            // collection — bail so the caller re-polls (do NOT build a partial set).
            let handle = untyped.clone().typed_debug_checked::<RonAsset<InjuryDef>>();
            let def = injury_defs.get(&handle)?;
            let Some(stem) = path
                .file_stem()
                .map(|stem| injury_key_from_stem(&stem.to_string_lossy()))
            else {
                continue;
            };
            // Validate the def's authored category against the owning subfolder
            // (organizational only): a mismatch WARNs but the injury is still loaded.
            warn_on_subfolder_mismatch(&path_str, &stem, def);
            registry.insert(InjuryName::new(stem), (**def).clone());
        } else if path_str.ends_with(".weighting.ron") {
            // A per-part `RonAsset<InjuryWeighting>`. One-frame race as above.
            let handle = untyped
                .clone()
                .typed_debug_checked::<RonAsset<InjuryWeighting>>();
            let weighting = weightings.get(&handle)?;
            authored_weightings.push((**weighting).clone());
        }
        // A member with neither infix is silently skipped (no usable injury type).
    }

    let tables = build_tables(&registry, &authored_weightings);
    audit_unweighted_injuries(&registry, &tables);
    Some((registry, tables))
}

/// Fold the authored per-category [`InjuryWeighting`]s into the [`InjuryTables`], resolving
/// each row's key against the registry (unknown → WARN + skip) and canonically sorting
/// the surviving rows so folder-enumeration order can never change the roll (GTW-437).
fn build_tables(registry: &InjuryRegistry, weightings: &[InjuryWeighting]) -> InjuryTables {
    let mut tables = InjuryTables::default();
    for weighting in weightings {
        // The three tabled severity buckets (None=graze, Fatal=death are never tabled).
        for (severity, rows) in [
            (Severity::Minor, &weighting.minor),
            (Severity::Major, &weighting.major),
            (Severity::Critical, &weighting.critical),
        ] {
            let mut resolved: Vec<WeightedInjuryEntry> = rows
                .iter()
                .filter(|row| {
                    // A weighting entry naming an UNKNOWN injury key → WARN + skip (no
                    // panic, no load failure — design rule C6).
                    let known = registry.contains(&row.injury);
                    if !known {
                        warn!(
                            "GDTF Load: weighting for {:?} names unknown injury key {:?}; \
                             skipping the row (it resolves to no injury)",
                            severity, &*row.injury,
                        );
                    }
                    known
                })
                .cloned()
                .collect();
            // CANONICAL SORT (by injury key, then weight): the authored Vec order — and
            // hence folder-enumeration order — can never change the built table / the
            // cumulative-weight pick (determinism, design fork #9).
            resolved.sort_by(|a, b| {
                a.injury
                    .cmp(&b.injury)
                    .then_with(|| a.weight.cmp(&b.weight))
            });
            if !resolved.is_empty() {
                tables.insert(
                    weighting.category,
                    severity,
                    WeightedInjuryTable::new(resolved),
                );
            }
        }
    }
    tables
}

/// Audit the missing-weighting rule: a registered injury referenced by NO weighting
/// bucket can never be rolled → `warn!` (a warning, never a load failure — rule C6).
fn audit_unweighted_injuries(registry: &InjuryRegistry, tables: &InjuryTables) {
    for (key, def) in registry.iter() {
        let weighted = tables
            .table_for_category(def.category, def.severity)
            .is_some_and(|table| table.iter().any(|row| row.injury == *key));
        if !weighted {
            warn!(
                "GDTF Load: injury {:?} ({:?}/{:?}) is in no weighting table; it can \
                 never be rolled",
                &**key, def.category, def.severity,
            );
        }
    }
}

/// `Update`: rebuild BOTH the [`InjuryRegistry`] and the [`InjuryTables`] in place on a
/// matching [`AssetEvent::Modified`](bevy::asset::AssetEvent::Modified) for ANY member
/// `*.injury.ron` OR `*.weighting.ron` — the GTW-374 LIVE injury hot-reload, the
/// injuries analogue of the GTW-257 weapons redrive (that family now rides the
/// GTW-570 generic seam), generalised to one folder → two resources / two asset types.
///
/// A folder load fans out into one `RonAsset<InjuryDef>` / `RonAsset<InjuryWeighting>`
/// asset PER file, and a hot edit fires an [`AssetEvent`](bevy::asset::AssetEvent)`::Modified`
/// for THAT member asset. This reacts to a `Modified` of EITHER type and rebuilds BOTH
/// resources from the PERSISTENT [`ActiveInjuriesFolderHandle`]'s member handles via
/// [`build_injury_data`] — the SAME builder the one-time resolve uses, so a live edit
/// yields the same resources a restart would. Overwriting via [`ResMut`] marks both
/// resources changed, so the next roll resolves against the edited data WITHOUT a
/// rebuild.
///
/// Guarded so it never panics before the load chain has resolved (pre-`Load`): it takes
/// the folder handle / the `Assets` collections / the [`InjuryRegistry`] +
/// [`InjuryTables`] resources as [`Option`]al borrows, draining BOTH readers and
/// returning early if any is missing (`bevy-traps.md` #1).
///
/// Param-only (`bevy-traps.md` #7): the two [`MessageReader`]s, the optional
/// [`AssetServer`] / folder handle / `Assets` / resource borrows.
#[expect(
    clippy::too_many_arguments,
    reason = "one folder carries two asset types, so the redrive reads two AssetEvent \
              readers + two Assets collections + two rebuilt resources; the weapons \
              mirror needs only one of each"
)]
pub(in crate::states::load) fn redrive_injuries_on_asset_event(
    mut def_events: MessageReader<AssetEvent<RonAsset<InjuryDef>>>,
    mut weighting_events: MessageReader<AssetEvent<RonAsset<InjuryWeighting>>>,
    asset_server: Option<Res<AssetServer>>,
    folder_handle: Option<Res<ActiveInjuriesFolderHandle>>,
    folders: Option<Res<Assets<LoadedFolder>>>,
    injury_defs: Option<Res<Assets<RonAsset<InjuryDef>>>>,
    weightings: Option<Res<Assets<RonAsset<InjuryWeighting>>>>,
    resources: Option<(ResMut<InjuryRegistry>, ResMut<InjuryTables>)>,
) {
    let (
        Some(asset_server),
        Some(folder_handle),
        Some(folders),
        Some(injury_defs),
        Some(weightings),
        Some((mut registry, mut tables)),
    ) = (
        asset_server,
        folder_handle,
        folders,
        injury_defs,
        weightings,
        resources,
    )
    else {
        // Drain BOTH readers so a pre-resolve event does not linger and re-fire once the
        // resources arrive; there is nothing to rebuild yet.
        def_events.clear();
        weighting_events.clear();
        return;
    };

    // Rebuild on ANY modified injury OR weighting member — a single rebuild from the
    // latest in-memory assets covers however many member events arrived this frame.
    let def_modified = def_events
        .read()
        .any(|event| matches!(event, AssetEvent::Modified { .. }));
    let weighting_modified = weighting_events
        .read()
        .any(|event| matches!(event, AssetEvent::Modified { .. }));
    if !def_modified && !weighting_modified {
        return;
    }

    let Some((rebuilt_registry, rebuilt_tables)) = build_injury_data(
        &asset_server,
        &folders,
        &injury_defs,
        &weightings,
        &folder_handle,
    ) else {
        // A member asset is mid-reload (not yet back in the collection) — leave the
        // existing resources until it settles; the next event rebuilds.
        return;
    };
    *registry = rebuilt_registry;
    *tables = rebuilt_tables;
    info!(
        "injury hot-reload: rebuilt InjuryRegistry + InjuryTables from `assets/content/injuries/` \
         ({} injuries, {} weighting buckets)",
        registry.len(),
        tables.len(),
    );
}

/// The injury KEY for a loaded injury file's stem — the stem with the dedicated
/// `.injury` infix stripped (GTW-437).
///
/// An injury file is `<key>.injury.ron`; Bevy's `file_stem()` yields `<key>.injury`,
/// so the KEY (the [`InjuryName`] a
/// [`WeightedInjuryEntry`](gdtf_battle_sim::injuries::WeightedInjuryEntry) references)
/// is that stem minus a trailing `.injury`. A stem without the infix is returned
/// unchanged (defensive — keeps a mis-named file's key its plain stem).
fn injury_key_from_stem(stem: &str) -> String {
    stem.strip_suffix(".injury").unwrap_or(stem).to_owned()
}

/// `warn!` if a loaded injury's authored [`category`](InjuryDef::category) differs from
/// the [`InjuryCategory`](gdtf_battle_sim::armor::InjuryCategory) the per-category
/// subfolder it lives in names (`injuries/<category>/<key>.injury.ron`, GTW-440 / GTW-453).
///
/// The subfolder is ORGANIZATIONAL only — the def's own `category` field is authoritative
/// (design fork #10) — so a mismatch is a content-authoring smell worth a warning, NEVER a
/// load failure: the injury is still loaded under its own field. Since GTW-440 the folders
/// are per-CATEGORY (`head` / `torso` / `arm` / `leg`); GTW-453 authors the category
/// DIRECTLY, so the comparison is the def's `category` against the subfolder's — an
/// `Arm`-declared injury under `arm/` is consistent, only a cross-category misfile WARNs.
fn warn_on_subfolder_mismatch(path_str: &str, key: &str, def: &InjuryDef) {
    let Some(subfolder) = subfolder_injury_category(path_str) else {
        // No recognised per-category subfolder (e.g. a flat layout) — nothing to compare.
        return;
    };
    if subfolder != def.category {
        warn!(
            "GDTF Load: injury {key:?} declares category {:?} but lives in the {:?} \
             subfolder; loading it under its authoritative field ({:?})",
            def.category, subfolder, def.category,
        );
    }
}

/// The [`InjuryCategory`](gdtf_battle_sim::armor::InjuryCategory) a
/// `injuries/<category>/…` path's per-category subfolder names, or [`None`] if the path
/// names no recognised category subfolder (GTW-440).
///
/// Maps the canonical per-category subfolder names (`head` / `torso` / `arm` / `leg`) to
/// their [`InjuryCategory`](gdtf_battle_sim::armor::InjuryCategory). The two arms / the
/// two legs share ONE folder each (the shared-pool restructure), so there is no
/// per-side subfolder anymore.
fn subfolder_injury_category(path_str: &str) -> Option<gdtf_battle_sim::armor::InjuryCategory> {
    use gdtf_battle_sim::armor::InjuryCategory;
    // Normalise to forward slashes so the match works on every platform.
    let normalised = path_str.replace('\\', "/");
    [
        ("/head/", InjuryCategory::Head),
        ("/torso/", InjuryCategory::Torso),
        ("/arm/", InjuryCategory::Arm),
        ("/leg/", InjuryCategory::Leg),
    ]
    .into_iter()
    .find_map(|(needle, category)| normalised.contains(needle).then_some(category))
}

#[cfg(test)]
mod test;
