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
//! `RonAsset<InjuryWeighting>`). Since GTW-654 the folder-walk BUILDER
//! ([`build_injury_data`]) and the GTW-582 per-file salvage fold live
//! host-agnostic in [`gdtf_content_families::injuries`] (the GTW-630 `validate`
//! precedent — the content editor's INJURY authoring mode runs the SAME
//! builder); THIS module keeps only the game host's systems: the `Load`-gated
//! resolve poll and the live redrive.

use bevy::{
    asset::{AssetEvent, AssetServer, Assets, LoadedFolder, RecursiveDependencyLoadState},
    prelude::{Commands, MessageReader, Res, ResMut, info},
};
use gdtf_assets::{ContentIntegrityReport, RonAsset, RonFolderSalvage};
use gdtf_battle_sim::injuries::{InjuryDef, InjuryRegistry, InjuryTables, InjuryWeighting};
use gdtf_content_families::injuries::{
    begin_injuries_salvage, build_injury_data, settle_injuries_salvage,
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
///   [`RecursiveDependencyLoadState::Failed`] it `warn!`s and begins a PER-FILE SALVAGE
///   (GTW-582 C4, through the ONE shared `gdtf_assets` salvage seam) — one salvage per
///   asset type (defs + weightings), settled together — so one malformed injury file no
///   longer vanishes every sibling; each malformed member is recorded as a loud
///   [`MalformedFile`](gdtf_assets::ContentFinding::MalformedFile) finding. A folder that
///   cannot be enumerated at all (missing directory) still fails closed to the EMPTY
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
#[expect(
    clippy::too_many_arguments,
    reason = "one folder carries two asset types, so the resolve reads two Assets \
              collections AND (GTW-582) two per-type salvage states + the shared report; \
              the single-type resolvers need only one of each"
)]
pub(super) fn resolve_injuries(
    commands: &mut Commands,
    asset_server: &AssetServer,
    folders: &Assets<LoadedFolder>,
    injury_defs: &Assets<RonAsset<InjuryDef>>,
    weightings: &Assets<RonAsset<InjuryWeighting>>,
    handles: &LoadHandles,
    salvage: (
        Option<&RonFolderSalvage<InjuryDef>>,
        Option<&RonFolderSalvage<InjuryWeighting>>,
    ),
    report: Option<&mut ContentIntegrityReport>,
) {
    // Salvage-poll path (GTW-582 C4): a prior frame's `Failed` began the per-file
    // salvages — settle BOTH (defs + weightings) before building either resource, so
    // the pair is still built atomically (the shared injuries-family fold).
    let (def_salvage, weighting_salvage) = salvage;
    if let (Some(def_salvage), Some(weighting_salvage)) = (def_salvage, weighting_salvage) {
        settle_injuries_salvage(
            commands,
            asset_server,
            injury_defs,
            weightings,
            def_salvage,
            weighting_salvage,
            report,
        );
        return;
    }

    let folder_state = asset_server.recursive_dependency_load_state(&*handles.injuries);

    // Failure path (GTW-582 C4): a Failed folder walk no longer empties the family —
    // salvage the members per-file (one salvage per asset type) so one malformed file
    // cannot vanish its siblings. Only a folder that cannot be enumerated at all still
    // fails closed to the EMPTY resources (the roll then fails closed — no injury
    // rolled — rather than crashing).
    if matches!(folder_state, RecursiveDependencyLoadState::Failed(_)) {
        begin_injuries_salvage(commands, asset_server);
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
/// the shared [`build_injury_data`] — the SAME builder the one-time resolve uses, so a
/// live edit yields the same resources a restart would. Overwriting via [`ResMut`] marks
/// both resources changed, so the next roll resolves against the edited data WITHOUT a
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

#[cfg(test)]
mod test;
