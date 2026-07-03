//! GTW-487 (child T05a of the GTW-476 data-model refactor): builds the UUID-keyed
//! [`TerrainDefRegistry`] + [`UuidThemeRegistry`] from the per-theme directory layout
//! `assets/content/terrain/<theme>/`, plus the LIVE hot-reload that rebuilds each on a
//! `*.terrain_def.ron` / `*.terrain_theme.ron` edit.
//!
//! **GTW-494 (child T08): the SOLE terrain / theme loaders.** These loaders were introduced
//! beside the legacy flat-dir `resolve_terrain` / `resolve_themes` (the GTW-394 / GTW-409
//! per-file terrain / theme model); GTW-494
//! RETIRED those legacy loaders (GTW-496 deleted their types), so these are now the ONLY
//! terrain / theme resolvers in the Load flow (the sim + procgen + presenter consume the new
//! registries as of GTW-491/492/493). The per-theme folder carries BOTH new asset types (one recursive
//! `load_folder` of `content/terrain/` fans every member out to the matching dedicated-extension
//! loader), so a single [`TerrainModelFolderHandle`] feeds both resolves and both redrives.
//!
//! **The dedicated `terrain_def.ron` / `terrain_theme.ron` extensions + the `TypeId` filter:**
//! Bevy 0.19 dispatches a `load_folder` member PURELY by extension
//! (`bevy_asset::server::loaders::get_by_path` → last loader registered for that extension,
//! directory-agnostic). The dedicated extensions keep dispatch unambiguous; ONE recursive
//! `load_folder("content/terrain")` fans out BOTH `RonAsset<TerrainDef>` and `RonAsset<UuidThemeDef>`
//! members, so each build helper filters by [`TypeId`] before typing the member.
//!
//! Each def carries its OWN UUID inside ([`TerrainDef::key`] / [`UuidThemeDef::key`]), so the
//! registries key by that def-owned UUID — the filename is irrelevant to the key.

use core::any::TypeId;

use bevy::{
    asset::{AssetEvent, AssetServer, Assets, LoadedFolder, RecursiveDependencyLoadState},
    prelude::{Commands, MessageReader, Res, ResMut, info, warn},
};
use gdtf_assets::RonAsset;
use gdtf_battle_sim::{
    level::{UuidThemeDef, UuidThemeRegistry},
    terrain::def::{TerrainDef, TerrainDefRegistry},
};

use crate::states::load::resources::{ActiveTerrainModelFolderHandle, LoadHandles};

/// GTW-487: builds the UUID-keyed [`TerrainDefRegistry`] from the loaded per-theme
/// `assets/content/terrain/` folder, keying by each def's OWN
/// [`TerrainUuid`](gdtf_battle_sim::terrain::def::TerrainUuid) (the filename is irrelevant).
/// GTW-494: the SOLE terrain resolve (the legacy flat-dir `resolve_terrain` was retired).
///
/// Called only while no [`TerrainDefRegistry`] resource exists yet (the caller's own-absence
/// guard), independently of the other load branches:
///
/// - Gates on the per-theme terrain-model folder's
///   [`RecursiveDependencyLoadState`]`::Loaded` (recursive, so every member under every
///   `<theme>/` subfolder is loaded). On [`RecursiveDependencyLoadState::Failed`] it `warn!`s
///   and inserts an EMPTY [`TerrainDefRegistry`] so `Load` always exits with one present and
///   never hangs on a bad folder (the ADR-0003 error-path safety-net; a consumer then fails
///   closed on a missing UUID rather than crashing).
/// - On success it reads the [`LoadedFolder`]'s member handles, types each as a
///   `RonAsset<TerrainDef>` (the other member type, `RonAsset<UuidThemeDef>`, is SKIPPED
///   here — the theme resolve picks it up), reads its [`TerrainDef`] out of the collection,
///   and inserts it under the def's OWN [`key`](TerrainDef::key). If ANY member that DOES
///   type as a `RonAsset<TerrainDef>` is not yet in its collection (the one-frame
///   loaded-but-not-yet-in-collection race), it returns WITHOUT inserting and retries next
///   frame — so a partial / empty registry is never published while a def member is pending.
/// - Else (still loading) it does nothing and is polled again next frame.
///
/// Resolves EMPTY against shipped content until the migration ticket (T06) authors the new
/// per-theme files — that empty state is the designed fail-closed state, NOT a failure.
pub(super) fn resolve_terrain_defs(
    commands: &mut Commands,
    asset_server: &AssetServer,
    folders: &Assets<LoadedFolder>,
    terrain_defs: &Assets<RonAsset<TerrainDef>>,
    handles: &LoadHandles,
) {
    let folder_state = asset_server.recursive_dependency_load_state(&*handles.terrain_model);

    if matches!(folder_state, RecursiveDependencyLoadState::Failed(_)) {
        warn!(
            "GDTF Load: the new per-theme `content/terrain/` folder failed to load; inserting an empty \
             TerrainDefRegistry (consumers fail closed on a missing terrain UUID)",
        );
        commands.insert_resource(TerrainDefRegistry::default());
        return;
    }

    if matches!(folder_state, RecursiveDependencyLoadState::Loaded) {
        let Some(registry) =
            build_terrain_def_registry(folders, terrain_defs, &handles.terrain_model)
        else {
            // A def member is loaded-but-not-yet-in-collection — retry next frame (the
            // system stays alive while the TerrainDefRegistry is absent).
            return;
        };

        // Insert the built registry — like the other Load registries it persists past
        // OnExit(Load) (it is NOT removed in cleanup).
        commands.insert_resource(registry);
        // Insert the PERSISTENT folder handle alongside the registries (idempotent — the
        // theme resolve inserts the same handle; whichever runs first wins, the other
        // overwrites with an equal handle), so the live hot-reload handlers can re-enumerate
        // the folder's members and holding it keeps every member asset loaded for the
        // file-watcher.
        commands.insert_resource(ActiveTerrainModelFolderHandle::new(
            (*handles.terrain_model).clone(),
        ));
    }
}

/// GTW-487: builds the UUID-keyed [`UuidThemeRegistry`] from the loaded per-theme
/// `assets/content/terrain/` folder, the theme mirror of [`resolve_terrain_defs`] — keying by each
/// def's OWN [`ThemeUuid`](gdtf_battle_sim::level::ThemeUuid).
///
/// Called only while no [`UuidThemeRegistry`] resource exists yet (the caller's own-absence
/// guard). Gates on the SAME folder's recursive load state as
/// [`resolve_terrain_defs`]; on failure inserts an EMPTY [`UuidThemeRegistry`] (the no-strand
/// safety-net); on success reads the folder's member handles, types each as a
/// `RonAsset<UuidThemeDef>` (the other member type is SKIPPED — the terrain-def resolve
/// handles it), and inserts it under the def's OWN [`key`](UuidThemeDef::key). Returns
/// without inserting (one-frame retry) if a theme member is not yet in its collection.
pub(super) fn resolve_theme_defs(
    commands: &mut Commands,
    asset_server: &AssetServer,
    folders: &Assets<LoadedFolder>,
    theme_defs: &Assets<RonAsset<UuidThemeDef>>,
    handles: &LoadHandles,
) {
    let folder_state = asset_server.recursive_dependency_load_state(&*handles.terrain_model);

    if matches!(folder_state, RecursiveDependencyLoadState::Failed(_)) {
        warn!(
            "GDTF Load: the new per-theme `content/terrain/` folder failed to load; inserting an empty \
             UuidThemeRegistry (consumers fail closed on a missing theme UUID)",
        );
        commands.insert_resource(UuidThemeRegistry::default());
        return;
    }

    if matches!(folder_state, RecursiveDependencyLoadState::Loaded) {
        let Some(registry) = build_theme_def_registry(folders, theme_defs, &handles.terrain_model)
        else {
            // A theme member is loaded-but-not-yet-in-collection — retry next frame.
            return;
        };

        commands.insert_resource(registry);
        commands.insert_resource(ActiveTerrainModelFolderHandle::new(
            (*handles.terrain_model).clone(),
        ));
    }
}

/// Build the UUID-keyed [`TerrainDefRegistry`] from a loaded per-theme `content/terrain/`
/// [`LoadedFolder`], or [`None`] if the folder (or any `RonAsset<TerrainDef>` member) is not
/// yet in its collection.
///
/// Shared by [`resolve_terrain_defs`] (the one-time `Load`-state build) and
/// [`redrive_terrain_defs_on_asset_event`] (the live rebuild on a hot edit), so both build
/// the registry IDENTICALLY: read the folder's member handles, type each as a
/// `RonAsset<TerrainDef>`, read its [`TerrainDef`] out of the collection, and key it by the
/// def's OWN [`key`](TerrainDef::key).
///
/// The folder is MIXED — it also holds `RonAsset<UuidThemeDef>` theme members — so each
/// member is FIRST filtered by its asset [`TypeId`]: a member whose type is not
/// `RonAsset<TerrainDef>` is SKIPPED (it is a theme member, handled by
/// [`build_theme_def_registry`]). This filter is mandatory — blindly typing a wrong-type
/// member as a `RonAsset<TerrainDef>` would trip the debug `typed_debug_checked` assert. Only
/// a member whose type DOES match but whose load has not yet landed forces the [`None`]
/// one-frame retry (the precise pending-vs-other-type disambiguation a single-type legacy
/// folder never needed).
fn build_terrain_def_registry(
    folders: &Assets<LoadedFolder>,
    terrain_defs: &Assets<RonAsset<TerrainDef>>,
    folder_handle: &bevy::asset::Handle<LoadedFolder>,
) -> Option<TerrainDefRegistry> {
    let folder = folders.get(folder_handle)?;

    let mut registry = TerrainDefRegistry::default();
    for untyped in &folder.handles {
        // Skip members that are not terrain defs (the theme members live in a DIFFERENT
        // Assets collection). Filtering by TypeId FIRST avoids the debug-assert panic a blind
        // `typed_debug_checked::<RonAsset<TerrainDef>>()` on a theme member would trip.
        if untyped.type_id() != TypeId::of::<RonAsset<TerrainDef>>() {
            continue;
        }
        let handle = untyped
            .clone()
            .typed_debug_checked::<RonAsset<TerrainDef>>();
        // A terrain-def member whose load has not yet landed — bail (do NOT publish a partial
        // registry) so the caller re-polls next frame.
        let def = terrain_defs.get(&handle)?;
        let def = (**def).clone();
        registry.insert(def.key, def);
    }
    Some(registry)
}

/// Build the UUID-keyed [`UuidThemeRegistry`] from a loaded per-theme `content/terrain/`
/// [`LoadedFolder`], or [`None`] if the folder is not yet in its collection — the theme
/// mirror of [`build_terrain_def_registry`].
///
/// Shared by [`resolve_theme_defs`] and [`redrive_theme_defs_on_asset_event`] so both build
/// the registry IDENTICALLY: read the folder's member handles, type each as a
/// `RonAsset<UuidThemeDef>`, read its [`UuidThemeDef`] out of the collection, and key it by
/// the def's OWN [`key`](UuidThemeDef::key). Like [`build_terrain_def_registry`], each member
/// is FIRST filtered by [`TypeId`] (a terrain-def member is SKIPPED) so a wrong-type member
/// never trips the debug `typed_debug_checked` assert; only a matching-type member mid-load
/// forces the [`None`] one-frame retry.
fn build_theme_def_registry(
    folders: &Assets<LoadedFolder>,
    theme_defs: &Assets<RonAsset<UuidThemeDef>>,
    folder_handle: &bevy::asset::Handle<LoadedFolder>,
) -> Option<UuidThemeRegistry> {
    let folder = folders.get(folder_handle)?;

    let mut registry = UuidThemeRegistry::default();
    for untyped in &folder.handles {
        // Skip non-theme members (terrain defs live in a different collection) — TypeId
        // filter FIRST to avoid the debug-assert panic on a wrong-type `typed_debug_checked`.
        if untyped.type_id() != TypeId::of::<RonAsset<UuidThemeDef>>() {
            continue;
        }
        let handle = untyped
            .clone()
            .typed_debug_checked::<RonAsset<UuidThemeDef>>();
        // A theme member mid-load — bail so the caller re-polls next frame.
        let def = theme_defs.get(&handle)?;
        let def = (**def).clone();
        registry.insert(def.key, def);
    }
    Some(registry)
}

/// `Update`: rebuild the [`TerrainDefRegistry`] in place on a matching
/// [`AssetEvent::Modified`](bevy::asset::AssetEvent::Modified) for any member
/// `assets/content/terrain/**/*.terrain_def.ron` — the GTW-487 LIVE terrain-def hot-reload, mirroring
/// the gang hot-reload pattern ([`redrive_gangs_on_asset_event`](super::gangs::redrive_gangs_on_asset_event)).
///
/// A folder load fans out into one `RonAsset<TerrainDef>` asset PER def file, and a hot edit
/// fires an [`AssetEvent`](bevy::asset::AssetEvent)`::Modified` for THAT member asset (not
/// the [`LoadedFolder`] handle), so this reacts to ANY
/// `AssetEvent<RonAsset<TerrainDef>>::Modified` and rebuilds the whole registry from the
/// PERSISTENT [`ActiveTerrainModelFolderHandle`]'s member handles via
/// [`build_terrain_def_registry`] — the SAME builder the one-time resolve uses.
///
/// Guarded so it never panics before the load chain has resolved (pre-`Load`): it takes the
/// folder handle / the `Assets` collection / the [`TerrainDefRegistry`] resource as
/// [`Option`]al borrows, draining the reader and returning early if any is missing
/// (`bevy-traps.md` #1).
///
/// Param-only (`bevy-traps.md` #7): the [`MessageReader`], the optional folder handle /
/// `Assets` / [`TerrainDefRegistry`] borrows.
pub(in crate::states::load) fn redrive_terrain_defs_on_asset_event(
    mut events: MessageReader<AssetEvent<RonAsset<TerrainDef>>>,
    folder_handle: Option<Res<ActiveTerrainModelFolderHandle>>,
    folders: Option<Res<Assets<LoadedFolder>>>,
    terrain_defs: Option<Res<Assets<RonAsset<TerrainDef>>>>,
    registry: Option<ResMut<TerrainDefRegistry>>,
) {
    let (Some(folder_handle), Some(folders), Some(terrain_defs), Some(mut registry)) =
        (folder_handle, folders, terrain_defs, registry)
    else {
        events.clear();
        return;
    };

    let modified = events
        .read()
        .any(|event| matches!(event, AssetEvent::Modified { .. }));
    if !modified {
        return;
    }

    let Some(rebuilt) = build_terrain_def_registry(&folders, &terrain_defs, &folder_handle) else {
        return;
    };
    *registry = rebuilt;
    info!(
        "terrain-def hot-reload: rebuilt TerrainDefRegistry from `assets/content/terrain/` ({} defs)",
        registry.len(),
    );
}

/// `Update`: rebuild the [`UuidThemeRegistry`] in place on a matching
/// [`AssetEvent::Modified`](bevy::asset::AssetEvent::Modified) for any member
/// `assets/content/terrain/**/*.terrain_theme.ron` — the GTW-487 LIVE new-theme-def hot-reload, the
/// theme mirror of [`redrive_terrain_defs_on_asset_event`].
///
/// Reacts to ANY `AssetEvent<RonAsset<UuidThemeDef>>::Modified` and rebuilds the whole
/// registry from the PERSISTENT [`ActiveTerrainModelFolderHandle`]'s member handles via
/// [`build_theme_def_registry`]. Guarded on `Option`al borrows (`bevy-traps.md` #1) so it is
/// a harmless no-op pre-`Load`.
pub(in crate::states::load) fn redrive_theme_defs_on_asset_event(
    mut events: MessageReader<AssetEvent<RonAsset<UuidThemeDef>>>,
    folder_handle: Option<Res<ActiveTerrainModelFolderHandle>>,
    folders: Option<Res<Assets<LoadedFolder>>>,
    theme_defs: Option<Res<Assets<RonAsset<UuidThemeDef>>>>,
    registry: Option<ResMut<UuidThemeRegistry>>,
) {
    let (Some(folder_handle), Some(folders), Some(theme_defs), Some(mut registry)) =
        (folder_handle, folders, theme_defs, registry)
    else {
        events.clear();
        return;
    };

    let modified = events
        .read()
        .any(|event| matches!(event, AssetEvent::Modified { .. }));
    if !modified {
        return;
    }

    let Some(rebuilt) = build_theme_def_registry(&folders, &theme_defs, &folder_handle) else {
        return;
    };
    *registry = rebuilt;
    info!(
        "theme-def hot-reload: rebuilt UuidThemeRegistry from `assets/content/terrain/` ({} themes)",
        registry.len(),
    );
}

#[cfg(test)]
mod test;
