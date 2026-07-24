//! GTW-489 (child T05c of the GTW-476 data-model refactor): builds the UUID-keyed
//! [`PrefabRegistry`] from the loaded `assets/content/maps/` folder, plus the LIVE hot-reload
//! that rebuilds it on a `*.prefab.ron` edit.
//!
//! **GTW-494 (child T08): the SOLE prefab loader.** This loader was introduced beside the
//! legacy flat-dir `resolve_prefabs` (the GTW-418 per-file prefab model); GTW-494 RETIRED
//! that legacy loader (GTW-496 deleted its types), so this is now the ONLY prefab resolver
//! in the Load flow (the procgen pipeline consumes [`PrefabRegistry`] as of GTW-492). The
//! fragments live under the `content/maps/` root: a recursive `load_folder` of
//! `content/maps/` fans every `*.prefab.ron` member to the dedicated-extension
//! [`RonAsset<PrefabSpec>`](gdtf_assets::RonAsset) loader, and a
//! [`PrefabsFolderHandle`](crate::states::load::resources::PrefabsFolderHandle) feeds
//! this resolve + redrive.
//!
//! **The dedicated `prefab.ron` extension + the `TypeId` filter:** Bevy 0.19 dispatches a
//! `load_folder` member PURELY by extension
//! (`bevy_asset::server::loaders::get_by_path` → last loader registered for that extension,
//! directory-agnostic). The `prefab.ron` extension keeps this dispatch unambiguous. The
//! [`TypeId`] filter in [`build_prefab_registry`] is DEFENSIVE only now (the `content/maps/`
//! folder is single-type after GTW-494), guarding against any future mixed-type member.
//!
//! **No opening validation:** this loader builds through the INFALLIBLE [`Prefab::new`] —
//! the schema carries no authored-opening field and no rejection path, because
//! inter-fragment connectivity is by-construction in the assembler (the 1-cell
//! `default_floor` margin every placement reserves; the old per-prefab opening machinery was
//! removed in GTW-497), not authored per-prefab. An openingless (zero-placement) prefab
//! is therefore INCLUDED in the registry (the GTW-488 design).

use core::any::TypeId;

use bevy::{
    asset::{AssetEvent, AssetServer, Assets, LoadedFolder, RecursiveDependencyLoadState},
    prelude::{Commands, MessageReader, Res, ResMut, info, warn},
};
use gdtf_assets::{
    ContentIntegrityReport, FindingFamily, RonAsset, RonFolderSalvage, RonSalvagePoll,
    begin_ron_folder_salvage, poll_ron_folder_salvage, report_malformed_members,
};
use gdtf_battle_sim::level::{Prefab, PrefabName, PrefabRegistry, PrefabSpec};
// GTW-634 C4: folder + extension from their single owning declarations (shared with the
// map editor's save path), so the salvage walk and the editor's write can never drift.
use gdtf_content_families::prefabs::{PREFAB_EXTENSION, PREFABS_FOLDER};

use crate::states::load::resources::{ActivePrefabsFolderHandle, LoadHandles};

/// GTW-489: builds the per-`(theme, size, role)` [`PrefabRegistry`] from the loaded
/// `assets/content/maps/` folder, bucketing UUID-keyed [`PrefabSpec`] fragments through the
/// INFALLIBLE [`Prefab::new`] (NO C6 edge-opening validation). GTW-494: the SOLE prefab
/// resolve (the legacy flat-dir `resolve_prefabs` was retired).
///
/// Called only while no [`PrefabRegistry`] resource exists yet (the caller's own-absence
/// guard), independently of the other load branches:
///
/// - Gates on the maps folder's
///   [`RecursiveDependencyLoadState`]`::Loaded` (recursive, so every prefab `.ron` under the
///   nested `<theme>/<size>/` subfolders is loaded). On
///   [`RecursiveDependencyLoadState::Failed`] it `warn!`s and inserts an EMPTY
///   [`PrefabRegistry`] so `Load` always exits with one present and never hangs on a bad
///   folder (the ADR-0003 error-path safety-net; the v2 assembler then has no fragments to
///   pack rather than crashing).
/// - On success it reads the [`LoadedFolder`]'s member handles, types each that IS a
///   `RonAsset<PrefabSpec>` (a `TypeId` filter, defensive — the new `maps/` root is a
///   single-type folder today, but the filter keeps the loader robust if it ever gains
///   another member type, the GTW-487 terrain-model precedent), reads its [`PrefabSpec`]
///   out of the collection, keys it by the asset path's file STEM with the dedicated
///   `.prefab` infix stripped (so `entry.prefab.ron` keys `entry` — the prefab NAME),
///   builds the prefab via the INFALLIBLE [`Prefab::new`] (no edge-opening rejection — an
///   openingless prefab is INCLUDED), and buckets it into the registry under its OWN spec's
///   `(theme, size, role)`. If ANY member that DOES type as a `RonAsset<PrefabSpec>` is not
///   yet in the collection (the one-frame loaded-but-not-yet-in-collection race), it returns
///   WITHOUT inserting and retries next frame — so a partial / empty registry is never
///   published while a member is pending. The registry holds the prefabs BY VALUE, so they
///   survive the folder handle being dropped on `OnExit(Load)`.
/// - Else (still loading) it does nothing and is polled again next frame.
///
/// Resolves EMPTY against shipped content until the migration ticket (T06) authors the
/// `*.prefab.ron` files — that empty state is the designed fail-closed state, NOT a
/// failure.
pub(super) fn resolve_prefabs(
    commands: &mut Commands,
    asset_server: &AssetServer,
    folders: &Assets<LoadedFolder>,
    prefab_specs: &Assets<RonAsset<PrefabSpec>>,
    handles: &LoadHandles,
    salvage: Option<&RonFolderSalvage<PrefabSpec>>,
    report: Option<&mut ContentIntegrityReport>,
) {
    // Salvage-poll path (GTW-582 C4): a prior frame's `Failed` began a per-file salvage —
    // settle it (bucket the loaded members, report the malformed ones).
    if let Some(salvage) = salvage {
        if let RonSalvagePoll::Settled { loaded, malformed } =
            poll_ron_folder_salvage(salvage, asset_server, prefab_specs)
        {
            let mut registry = PrefabRegistry::default();
            for member in &loaded {
                let Some(stem) = std::path::Path::new(member.path.as_str())
                    .file_stem()
                    .map(|stem| prefab_name_from_stem(&stem.to_string_lossy()))
                else {
                    continue;
                };
                registry.insert(Prefab::new(PrefabName::new(stem), (**member.spec).clone()));
            }
            report_malformed_members(
                report,
                &FindingFamily::new("PrefabRegistry".to_owned()),
                malformed,
            );
            commands.insert_resource(registry);
        }
        return;
    }

    let folder_state = asset_server.recursive_dependency_load_state(&*handles.prefabs);

    // Failure path (GTW-582 C4): a Failed folder walk no longer empties the family — salvage
    // the members per-file so one malformed fragment cannot vanish its siblings. Only a
    // folder that cannot be enumerated at all still fails closed to the EMPTY registry, so
    // Load always exits with one present (the assembler then has no fragments rather than
    // crashing).
    if matches!(folder_state, RecursiveDependencyLoadState::Failed(_)) {
        match begin_ron_folder_salvage::<PrefabSpec>(asset_server, PREFABS_FOLDER, PREFAB_EXTENSION)
        {
            Ok(salvage) if !salvage.is_empty() => {
                warn!(
                    "GDTF Load: the `maps` folder failed to load; salvaging its fragments \
                     per-file into the PrefabRegistry",
                );
                commands.insert_resource(salvage);
            }
            _ => {
                warn!(
                    "GDTF Load: the `maps` folder failed to load; inserting an empty \
                     PrefabRegistry (the assembler will have no fragments to pack)",
                );
                commands.insert_resource(PrefabRegistry::default());
            }
        }
        return;
    }

    // Success path: once every prefab file in the folder is loaded, read the LoadedFolder's
    // member handles and build the bucketed registry.
    if matches!(folder_state, RecursiveDependencyLoadState::Loaded) {
        let Some(registry) =
            build_prefab_registry(asset_server, folders, prefab_specs, &handles.prefabs)
        else {
            // Loaded-but-not-yet-in-collection (the folder, or a member spec) — retry next
            // frame (the system stays alive while the PrefabRegistry is absent).
            return;
        };

        // Insert the built registry — like the other Load registries it persists past
        // OnExit(Load) (it is NOT removed in cleanup), because the assembler reads it.
        commands.insert_resource(registry);
        // GTW-489: insert the PERSISTENT folder handle alongside the registry — it survives
        // OnExit(Load) so the live hot-reload handler can re-enumerate the folder's member
        // handles to rebuild the registry on a `*.prefab.ron` edit, and holding it keeps
        // every member prefab asset loaded for the file-watcher.
        commands.insert_resource(ActivePrefabsFolderHandle::new((*handles.prefabs).clone()));
    }
}

/// Build the bucketed [`PrefabRegistry`] from a loaded `maps/` [`LoadedFolder`], or
/// [`None`] if the folder (or any `RonAsset<PrefabSpec>` member) is not yet in its
/// collection.
///
/// Shared by [`resolve_prefabs`] (the one-time `Load`-state build) and the GTW-489
/// [`redrive_prefabs_on_asset_event`] (the live rebuild on a hot edit), so both build the
/// registry IDENTICALLY: read the folder's member handles, type each that IS a
/// `RonAsset<PrefabSpec>`, read its [`PrefabSpec`] out of the collection, key it by the
/// asset path's file STEM with the dedicated `.prefab` infix stripped, build it through
/// the INFALLIBLE [`Prefab::new`] (NO edge-opening rejection — an openingless prefab is
/// INCLUDED), and bucket it by its own `(theme, size, role)`.
///
/// Each member is FIRST filtered by its asset [`TypeId`]: a member whose type is not
/// `RonAsset<PrefabSpec>` is SKIPPED. The `maps/` root is a SINGLE-TYPE folder today
/// (only `*.prefab.ron`), so the filter is defensive — but it keeps the loader robust if
/// the root ever gains another member type, and a blind `typed_debug_checked` on a wrong-type
/// member would trip the debug assert (the GTW-487 terrain-model precedent). Only a member
/// whose type DOES match but whose load has not yet landed forces the [`None`] one-frame
/// retry.
fn build_prefab_registry(
    asset_server: &AssetServer,
    folders: &Assets<LoadedFolder>,
    prefab_specs: &Assets<RonAsset<PrefabSpec>>,
    folder_handle: &bevy::asset::Handle<LoadedFolder>,
) -> Option<PrefabRegistry> {
    let folder = folders.get(folder_handle)?;

    let mut registry = PrefabRegistry::default();
    for untyped in &folder.handles {
        // Skip members that are not prefabs (defensive — the `maps/` root is a single-type
        // folder today). Filtering by TypeId FIRST avoids the debug-assert panic a blind
        // `typed_debug_checked::<RonAsset<PrefabSpec>>()` on a wrong-type member would trip.
        if untyped.type_id() != TypeId::of::<RonAsset<PrefabSpec>>() {
            continue;
        }
        let handle = untyped
            .clone()
            .typed_debug_checked::<RonAsset<PrefabSpec>>();
        // A member whose load has not yet landed — bail (do NOT publish a partial
        // registry) so the caller re-polls next frame.
        let spec = prefab_specs.get(&handle)?;
        // Key by the asset path's file STEM with the dedicated `.prefab` infix stripped:
        // `entry.prefab.ron`'s `file_stem()` is `entry.prefab`, whose prefab NAME is
        // `entry`. A handle with no resolvable path / stem is skipped defensively (it would
        // carry no usable name).
        let Some(stem) = asset_server.get_path(untyped.id()).and_then(|path| {
            path.path()
                .file_stem()
                .map(|stem| prefab_name_from_stem(&stem.to_string_lossy()))
        }) else {
            continue;
        };
        let name = PrefabName::new(stem);
        // INFALLIBLE: the schema has no edge-opening validation, so an openingless
        // (zero-placement) prefab is INCLUDED (contrast the legacy resolve's C6 reject).
        registry.insert(Prefab::new(name, (**spec).clone()));
    }
    Some(registry)
}

/// `Update`: rebuild the [`PrefabRegistry`] in place on a matching
/// [`AssetEvent::Modified`](bevy::asset::AssetEvent::Modified) for any member
/// `assets/content/maps/**/*.prefab.ron` — the GTW-489 LIVE prefab hot-reload, mirroring
/// the GTW-415 gang hot-reload pattern (that family now rides the GTW-570 generic
/// content-family registration).
///
/// A folder load fans out into one `RonAsset<PrefabSpec>` asset PER file, and a hot edit
/// fires an [`AssetEvent`](bevy::asset::AssetEvent)`::Modified` for THAT member asset (not
/// the [`LoadedFolder`] handle), so this reacts to ANY
/// `AssetEvent<RonAsset<PrefabSpec>>::Modified` and rebuilds the whole registry from the
/// PERSISTENT [`ActivePrefabsFolderHandle`]'s member handles via [`build_prefab_registry`]
/// — the SAME builder the one-time resolve uses.
///
/// Guarded so it never panics before the load chain has resolved (pre-`Load`): it takes the
/// folder handle / the `Assets` collections / the [`PrefabRegistry`] resource as
/// [`Option`]al borrows, draining the reader and returning early if any is missing
/// (`bevy-traps.md` #1).
///
/// Param-only (`bevy-traps.md` #7): the [`MessageReader`], the optional [`AssetServer`] /
/// folder handle / `Assets` / [`PrefabRegistry`] borrows.
pub(in crate::states::load) fn redrive_prefabs_on_asset_event(
    mut events: MessageReader<AssetEvent<RonAsset<PrefabSpec>>>,
    asset_server: Option<Res<AssetServer>>,
    folder_handle: Option<Res<ActivePrefabsFolderHandle>>,
    folders: Option<Res<Assets<LoadedFolder>>>,
    prefab_specs: Option<Res<Assets<RonAsset<PrefabSpec>>>>,
    registry: Option<ResMut<PrefabRegistry>>,
) {
    let (
        Some(asset_server),
        Some(folder_handle),
        Some(folders),
        Some(prefab_specs),
        Some(mut registry),
    ) = (asset_server, folder_handle, folders, prefab_specs, registry)
    else {
        // Drain the reader so a pre-resolve event does not linger and re-fire once the
        // resources arrive; there is nothing to rebuild yet.
        events.clear();
        return;
    };

    // Rebuild on ANY modified prefab member — a single rebuild from the latest in-memory
    // specs covers however many member events arrived this frame.
    let modified = events
        .read()
        .any(|event| matches!(event, AssetEvent::Modified { .. }));
    if !modified {
        return;
    }

    let Some(rebuilt) =
        build_prefab_registry(&asset_server, &folders, &prefab_specs, &folder_handle)
    else {
        // A member spec is mid-reload (not yet back in the collection) — leave the existing
        // registry until it settles; the next event rebuilds.
        return;
    };
    *registry = rebuilt;
    info!(
        "prefab hot-reload: rebuilt PrefabRegistry from `assets/content/maps/` ({} prefabs)",
        registry.len(),
    );
}

/// The prefab NAME for a loaded prefab file's stem — the stem with the dedicated
/// `.prefab` infix stripped (GTW-489, GTW-557).
///
/// A prefab file is `<name>.prefab.ron`; Bevy's `file_stem()` yields `<name>.prefab`,
/// so the NAME is that stem minus a trailing `.prefab`. A stem without the infix is
/// returned unchanged (defensive — keeps a mis-named file's name its plain stem).
fn prefab_name_from_stem(stem: &str) -> String {
    stem.strip_suffix(".prefab").unwrap_or(stem).to_owned()
}

#[cfg(test)]
mod test;
