//! GTW-489 (child T05c of the GTW-476 data-model refactor): builds the UUID-keyed
//! [`PrefabRegistry2`] from the loaded `assets/content/maps/` folder, plus the LIVE hot-reload
//! that rebuilds it on a `*.prefab_v2.ron` edit.
//!
//! **GTW-494 (child T08): the SOLE prefab loader.** This loader was introduced beside the
//! legacy flat-dir `resolve_prefabs` (the GTW-418 per-file prefab model); GTW-494 RETIRED
//! that legacy loader (GTW-496 deleted its types), so this is now the ONLY prefab resolver
//! in the Load flow (the procgen pipeline consumes [`PrefabRegistry2`] as of GTW-492). The
//! v2 fragments live under the
//! `content/maps/` root: a recursive `load_folder` of `content/maps/` fans every
//! `*.prefab_v2.ron` member to
//! the dedicated-extension [`RonAsset<PrefabSpecV2>`](gdtf_assets::RonAsset) loader, and a
//! [`PrefabsV2FolderHandle`](crate::states::load::resources::PrefabsV2FolderHandle) feeds
//! this resolve + redrive.
//!
//! **The dedicated `prefab_v2.ron` extension + the `TypeId` filter:** Bevy 0.19 dispatches a
//! `load_folder` member PURELY by extension
//! (`bevy_asset::server::loaders::get_by_path` → last loader registered for that extension,
//! directory-agnostic). The `prefab_v2.ron` extension keeps this dispatch unambiguous. The
//! [`TypeId`] filter in [`build_prefab_v2_registry`] is DEFENSIVE only now (the `content/maps/`
//! folder is single-type after GTW-494), guarding against any future mixed-type member.
//!
//! **No opening validation:** this loader builds through the INFALLIBLE [`Prefab2::new`] —
//! the v2 schema carries no authored-opening field and no rejection path, because
//! inter-fragment connectivity is by-construction in the v2 assembler (the 1-cell
//! `default_floor` seam every placement reserves; the old per-prefab opening machinery was
//! removed in GTW-497), not authored per-prefab. An openingless (zero-placement) v2 prefab
//! is therefore INCLUDED in the registry (the GTW-488 design).

use core::any::TypeId;

use bevy::{
    asset::{AssetEvent, AssetServer, Assets, LoadedFolder, RecursiveDependencyLoadState},
    prelude::{Commands, MessageReader, Res, ResMut, info, warn},
};
use gdtf_assets::RonAsset;
use gdtf_battle_sim::level::{Prefab2, PrefabName, PrefabRegistry2, PrefabSpecV2};

use crate::states::load::resources::{ActivePrefabsV2FolderHandle, LoadHandles};

/// GTW-489: builds the per-`(theme, size, role)` [`PrefabRegistry2`] from the loaded
/// `assets/content/maps/` folder, bucketing UUID-keyed [`PrefabSpecV2`] fragments through the
/// INFALLIBLE [`Prefab2::new`] (NO C6 edge-opening validation). GTW-494: the SOLE prefab
/// resolve (the legacy flat-dir `resolve_prefabs` was retired).
///
/// Called only while no [`PrefabRegistry2`] resource exists yet (the caller's own-absence
/// guard), independently of the other load branches:
///
/// - Gates on the maps folder's
///   [`RecursiveDependencyLoadState`]`::Loaded` (recursive, so every prefab `.ron` under the
///   nested `<theme>/<size>/` subfolders is loaded). On
///   [`RecursiveDependencyLoadState::Failed`] it `warn!`s and inserts an EMPTY
///   [`PrefabRegistry2`] so `Load` always exits with one present and never hangs on a bad
///   folder (the ADR-0003 error-path safety-net; the v2 assembler then has no fragments to
///   pack rather than crashing).
/// - On success it reads the [`LoadedFolder`]'s member handles, types each that IS a
///   `RonAsset<PrefabSpecV2>` (a `TypeId` filter, defensive — the new `maps/` root is a
///   single-type folder today, but the filter keeps the loader robust if it ever gains
///   another member type, the GTW-487 terrain-model precedent), reads its [`PrefabSpecV2`]
///   out of the collection, keys it by the asset path's file STEM with the dedicated
///   `.prefab_v2` infix stripped (so
///   `entry.prefab_v2.ron` keys `entry` — the prefab NAME), builds the prefab via the
///   INFALLIBLE [`Prefab2::new`] (no edge-opening rejection — an openingless prefab is
///   INCLUDED), and buckets it into the registry under its OWN spec's `(theme, size, role)`.
///   If ANY member that DOES type as a `RonAsset<PrefabSpecV2>` is not yet in the collection
///   (the one-frame loaded-but-not-yet-in-collection race), it returns WITHOUT inserting and
///   retries next frame — so a partial / empty registry is never published while a v2 member
///   is pending. The registry holds the prefabs BY VALUE, so they survive the folder handle
///   being dropped on `OnExit(Load)`.
/// - Else (still loading) it does nothing and is polled again next frame.
///
/// Resolves EMPTY against shipped content until the migration ticket (T06) authors the
/// `*.prefab_v2.ron` files — that empty state is the designed fail-closed state, NOT a
/// failure.
pub(super) fn resolve_prefabs_v2(
    commands: &mut Commands,
    asset_server: &AssetServer,
    folders: &Assets<LoadedFolder>,
    prefab_specs: &Assets<RonAsset<PrefabSpecV2>>,
    handles: &LoadHandles,
) {
    let folder_state = asset_server.recursive_dependency_load_state(&*handles.prefabs_v2);

    // Failure path: a bad/missing maps folder must not hang the app. Warn and insert an EMPTY
    // registry so Load always exits with one present (the v2 assembler then has no fragments
    // rather than crashing).
    if matches!(folder_state, RecursiveDependencyLoadState::Failed(_)) {
        warn!(
            "GDTF Load: the `maps` folder failed to load; inserting an empty PrefabRegistry2 \
             (the v2 assembler will have no fragments to pack)",
        );
        commands.insert_resource(PrefabRegistry2::default());
        return;
    }

    // Success path: once every prefab file in the folder is loaded, read the LoadedFolder's
    // member handles and build the bucketed registry.
    if matches!(folder_state, RecursiveDependencyLoadState::Loaded) {
        let Some(registry) =
            build_prefab_v2_registry(asset_server, folders, prefab_specs, &handles.prefabs_v2)
        else {
            // Loaded-but-not-yet-in-collection (the folder, or a v2 member spec) — retry next
            // frame (the system stays alive while the PrefabRegistry2 is absent).
            return;
        };

        // Insert the built registry — like the other Load registries it persists past
        // OnExit(Load) (it is NOT removed in cleanup), because the v2 assembler reads it.
        commands.insert_resource(registry);
        // GTW-489: insert the PERSISTENT folder handle alongside the registry — it survives
        // OnExit(Load) so the live hot-reload handler can re-enumerate the folder's member
        // handles to rebuild the registry on a `*.prefab_v2.ron` edit, and holding it keeps
        // every member v2 prefab asset loaded for the file-watcher.
        commands.insert_resource(ActivePrefabsV2FolderHandle::new(
            (*handles.prefabs_v2).clone(),
        ));
    }
}

/// Build the bucketed [`PrefabRegistry2`] from a loaded `maps/` [`LoadedFolder`], or
/// [`None`] if the folder (or any `RonAsset<PrefabSpecV2>` member) is not yet in its
/// collection — the v2 mirror of `build_prefab_registry` (in the sibling `prefabs` module).
///
/// Shared by [`resolve_prefabs_v2`] (the one-time `Load`-state build) and the GTW-489
/// [`redrive_prefabs_v2_on_asset_event`] (the live rebuild on a hot edit), so both build the
/// registry IDENTICALLY: read the folder's member handles, type each that IS a
/// `RonAsset<PrefabSpecV2>`, read its [`PrefabSpecV2`] out of the collection, key it by the
/// asset path's file STEM with the dedicated `.prefab_v2` infix stripped, build it through
/// the INFALLIBLE [`Prefab2::new`] (NO edge-opening rejection — an openingless prefab is
/// INCLUDED), and bucket it by its own `(theme, size, role)`.
///
/// Each member is FIRST filtered by its asset [`TypeId`]: a member whose type is not
/// `RonAsset<PrefabSpecV2>` is SKIPPED. The new `maps/` root is a SINGLE-TYPE folder today
/// (only `*.prefab_v2.ron`), so the filter is defensive — but it keeps the loader robust if
/// the root ever gains another member type, and a blind `typed_debug_checked` on a wrong-type
/// member would trip the debug assert (the GTW-487 terrain-model precedent). Only a member
/// whose type DOES match but whose load has not yet landed forces the [`None`] one-frame
/// retry.
fn build_prefab_v2_registry(
    asset_server: &AssetServer,
    folders: &Assets<LoadedFolder>,
    prefab_specs: &Assets<RonAsset<PrefabSpecV2>>,
    folder_handle: &bevy::asset::Handle<LoadedFolder>,
) -> Option<PrefabRegistry2> {
    let folder = folders.get(folder_handle)?;

    let mut registry = PrefabRegistry2::default();
    for untyped in &folder.handles {
        // Skip members that are not v2 prefabs (defensive — the new `maps/` root is a
        // single-type folder today). Filtering by TypeId FIRST avoids the debug-assert panic
        // a blind `typed_debug_checked::<RonAsset<PrefabSpecV2>>()` on a wrong-type member
        // would trip.
        if untyped.type_id() != TypeId::of::<RonAsset<PrefabSpecV2>>() {
            continue;
        }
        let handle = untyped
            .clone()
            .typed_debug_checked::<RonAsset<PrefabSpecV2>>();
        // A v2 member whose load has not yet landed — bail (do NOT publish a partial
        // registry) so the caller re-polls next frame.
        let spec = prefab_specs.get(&handle)?;
        // Key by the asset path's file STEM with the dedicated `.prefab_v2` infix stripped:
        // `entry.prefab_v2.ron`'s `file_stem()` is `entry.prefab_v2`, whose prefab NAME is
        // `entry`. A handle with no resolvable path / stem is skipped defensively (it would
        // carry no usable name).
        let Some(stem) = asset_server.get_path(untyped.id()).and_then(|path| {
            path.path()
                .file_stem()
                .map(|stem| prefab_v2_name_from_stem(&stem.to_string_lossy()))
        }) else {
            continue;
        };
        let name = PrefabName::new(stem);
        // INFALLIBLE: the v2 schema has no edge-opening validation, so an openingless
        // (zero-placement) prefab is INCLUDED (contrast the legacy resolve's C6 reject).
        registry.insert(Prefab2::new(name, (**spec).clone()));
    }
    Some(registry)
}

/// `Update`: rebuild the [`PrefabRegistry2`] in place on a matching
/// [`AssetEvent::Modified`](bevy::asset::AssetEvent::Modified) for any member
/// `assets/content/maps/**/*.prefab_v2.ron` — the GTW-489 LIVE v2 prefab hot-reload, mirroring the
/// gang hot-reload pattern ([`redrive_gangs_on_asset_event`](super::gangs::redrive_gangs_on_asset_event)).
///
/// A folder load fans out into one `RonAsset<PrefabSpecV2>` asset PER v2 file, and a hot edit
/// fires an [`AssetEvent`](bevy::asset::AssetEvent)`::Modified` for THAT member asset (not
/// the [`LoadedFolder`] handle), so this reacts to ANY
/// `AssetEvent<RonAsset<PrefabSpecV2>>::Modified` and rebuilds the whole registry from the
/// PERSISTENT [`ActivePrefabsV2FolderHandle`]'s member handles via
/// [`build_prefab_v2_registry`] — the SAME builder the one-time resolve uses.
///
/// Guarded so it never panics before the load chain has resolved (pre-`Load`): it takes the
/// folder handle / the `Assets` collections / the [`PrefabRegistry2`] resource as
/// [`Option`]al borrows, draining the reader and returning early if any is missing
/// (`bevy-traps.md` #1).
///
/// Param-only (`bevy-traps.md` #7): the [`MessageReader`], the optional [`AssetServer`] /
/// folder handle / `Assets` / [`PrefabRegistry2`] borrows.
pub(in crate::states::load) fn redrive_prefabs_v2_on_asset_event(
    mut events: MessageReader<AssetEvent<RonAsset<PrefabSpecV2>>>,
    asset_server: Option<Res<AssetServer>>,
    folder_handle: Option<Res<ActivePrefabsV2FolderHandle>>,
    folders: Option<Res<Assets<LoadedFolder>>>,
    prefab_specs: Option<Res<Assets<RonAsset<PrefabSpecV2>>>>,
    registry: Option<ResMut<PrefabRegistry2>>,
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

    // Rebuild on ANY modified v2 prefab member — a single rebuild from the latest in-memory
    // specs covers however many member events arrived this frame.
    let modified = events
        .read()
        .any(|event| matches!(event, AssetEvent::Modified { .. }));
    if !modified {
        return;
    }

    let Some(rebuilt) =
        build_prefab_v2_registry(&asset_server, &folders, &prefab_specs, &folder_handle)
    else {
        // A member spec is mid-reload (not yet back in the collection) — leave the existing
        // registry until it settles; the next event rebuilds.
        return;
    };
    *registry = rebuilt;
    info!(
        "prefab-v2 hot-reload: rebuilt PrefabRegistry2 from `assets/content/maps/` ({} prefabs)",
        registry.len(),
    );
}

/// The prefab NAME for a loaded v2 prefab file's stem — the stem with the dedicated
/// `.prefab_v2` infix stripped (GTW-489).
///
/// A v2 prefab file is `<name>.prefab_v2.ron`; Bevy's `file_stem()` yields `<name>.prefab_v2`,
/// so the NAME is that stem minus a trailing `.prefab_v2`. A stem without the infix is
/// returned unchanged (defensive — keeps a mis-named file's name its plain stem).
fn prefab_v2_name_from_stem(stem: &str) -> String {
    stem.strip_suffix(".prefab_v2").unwrap_or(stem).to_owned()
}

#[cfg(test)]
mod test;
