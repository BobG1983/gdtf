//! GTW-545: builds the stem-keyed [`FieldDefRegistry`] (the area-damage-field catalog) from the
//! loaded `assets/content/fields/` folder, plus the LIVE hot-reload that rebuilds it on a
//! `*.field.ron` edit — the armor-loader mirror (`resolve_armor`).

use bevy::{
    asset::{AssetEvent, AssetServer, Assets, LoadedFolder, RecursiveDependencyLoadState},
    prelude::{Commands, MessageReader, Res, ResMut, info, warn},
};
use gdtf_assets::RonAsset;
use gdtf_battle_sim::{FieldDef, FieldDefRegistry, FieldKey};

use crate::states::load::resources::{ActiveFieldsFolderHandle, LoadHandles};

/// GTW-545: builds the stem-keyed [`FieldDefRegistry`] from the loaded
/// `assets/content/fields/` folder, mirroring the GTW-269 armor resolve shape exactly (the
/// area-damage-field mirror of [`resolve_armor`](super::armor::resolve_armor)).
///
/// Called only while no [`FieldDefRegistry`] resource exists yet (the caller's own-absence
/// guard), independently of the other branches:
///
/// - Gates on the fields folder's [`RecursiveDependencyLoadState`]`::Loaded`. On
///   [`RecursiveDependencyLoadState::Failed`] it `warn!`s and inserts an EMPTY
///   [`FieldDefRegistry`] so `Load` always exits with one present and never hangs on a bad
///   folder (the ADR-0003 error-path safety-net; the setup then fails closed with
///   `FieldNotFound` on a missing field key rather than crashing).
/// - On success it reads the [`LoadedFolder`]'s member handles, types each as a
///   `RonAsset<FieldDef>`, reads its [`FieldDef`] out of the collection, keys it by the asset
///   path's file STEM with the dedicated `.field` infix stripped (so
///   `toxic_waste_pool.field.ron` keys `toxic_waste_pool`), and inserts every
///   `(FieldKey, FieldDef)`. If ANY member def is not yet in the collection (the one-frame
///   loaded-but-not-yet-in-collection race), it returns WITHOUT inserting and retries next
///   frame. The registry holds the defs BY VALUE, so they survive the folder handle being
///   dropped on `OnExit(Load)`.
/// - Else (still loading) it does nothing and is polled again next frame.
pub(super) fn resolve_fields(
    commands: &mut Commands,
    asset_server: &AssetServer,
    folders: &Assets<LoadedFolder>,
    field_defs: &Assets<RonAsset<FieldDef>>,
    handles: &LoadHandles,
) {
    let folder_state = asset_server.recursive_dependency_load_state(&*handles.fields);

    // Failure path: a bad/missing fields folder must not hang the app. Warn and insert an
    // EMPTY registry so Load always exits with one present (the setup then fails closed with
    // FieldNotFound on a missing field key rather than crashing).
    if matches!(folder_state, RecursiveDependencyLoadState::Failed(_)) {
        warn!(
            "GDTF Load: the `fields` folder failed to load; inserting an empty FieldDefRegistry \
             (battles seeding a field will fail closed on a missing field key)",
        );
        commands.insert_resource(FieldDefRegistry::default());
        return;
    }

    // Success path: once every field file in the folder is loaded, read the LoadedFolder's
    // member handles and build the stem-keyed registry.
    if matches!(folder_state, RecursiveDependencyLoadState::Loaded) {
        let Some(registry) =
            build_field_registry(asset_server, folders, field_defs, &handles.fields)
        else {
            // Loaded-but-not-yet-in-collection — retry next frame (the system stays alive
            // while the FieldDefRegistry is absent).
            return;
        };

        // Insert the built registry — it persists past OnExit(Load) (NOT removed in cleanup),
        // because the battle setup reads it to seed fields.
        commands.insert_resource(registry);
        // Insert the PERSISTENT folder handle alongside the registry so the live hot-reload
        // handler can re-enumerate the folder's member handles to rebuild the catalog on a
        // `*.field.ron` edit, and holding it keeps every member field asset loaded for the
        // file-watcher.
        commands.insert_resource(ActiveFieldsFolderHandle::new((*handles.fields).clone()));
    }
}

/// Build the stem-keyed [`FieldDefRegistry`] from a loaded `fields/` [`LoadedFolder`], or
/// [`None`] if the folder (or any member def) is not yet in its collection — the field mirror
/// of `build_armor_registry`.
///
/// Shared by [`resolve_fields`] (the one-time `Load`-state build) and
/// [`redrive_fields_on_asset_event`] (the live rebuild on a hot edit), so both build the
/// registry IDENTICALLY. Returns [`None`] (do NOT publish a partial registry) if the folder or
/// any member def is not yet in its collection — the caller retries next frame.
fn build_field_registry(
    asset_server: &AssetServer,
    folders: &Assets<LoadedFolder>,
    field_defs: &Assets<RonAsset<FieldDef>>,
    folder_handle: &bevy::asset::Handle<LoadedFolder>,
) -> Option<FieldDefRegistry> {
    let folder = folders.get(folder_handle)?;

    let mut registry = FieldDefRegistry::default();
    for untyped in &folder.handles {
        // Type the untyped member handle as a RonAsset<FieldDef> and read its def out of the
        // collection.
        let handle = untyped.clone().typed_debug_checked::<RonAsset<FieldDef>>();
        // One-frame loaded-but-not-yet-in-collection race: a member def is not in the
        // collection yet. Bail (do NOT build a partial registry) so the caller re-polls.
        let def = field_defs.get(&handle)?;
        // Key by the asset path's file STEM with the dedicated `.field` infix stripped:
        // `toxic_waste_pool.field.ron`'s `file_stem()` is `toxic_waste_pool.field`, whose field
        // KEY is `toxic_waste_pool`. A handle with no resolvable path / stem is skipped
        // defensively (it would carry no usable key).
        let Some(stem) = asset_server.get_path(untyped.id()).and_then(|path| {
            path.path()
                .file_stem()
                .map(|stem| field_key_from_stem(&stem.to_string_lossy()))
        }) else {
            continue;
        };
        registry.insert(FieldKey::new(stem), (**def).clone());
    }
    Some(registry)
}

/// `Update`: rebuild the [`FieldDefRegistry`] in place on a matching
/// [`AssetEvent::Modified`](bevy::asset::AssetEvent::Modified) for any member
/// `assets/content/fields/*.field.ron` — the LIVE field hot-reload, the armor mirror of
/// `redrive_armor_on_asset_event`.
///
/// A folder load fans out into one `RonAsset<FieldDef>` asset PER file, and a hot edit fires an
/// [`AssetEvent`](bevy::asset::AssetEvent)`::Modified` for THAT member asset (not the
/// [`LoadedFolder`] handle), so this reacts to ANY `AssetEvent<RonAsset<FieldDef>>::Modified`
/// and rebuilds the whole catalog from the PERSISTENT [`ActiveFieldsFolderHandle`]'s member
/// handles via [`build_field_registry`]. Overwriting via [`ResMut`] marks the registry changed,
/// so the next battle setup resolves against the edited defs.
///
/// Guarded so it never panics before the load chain has resolved (pre-`Load`): it takes the
/// folder handle / the `Assets` collections / the [`FieldDefRegistry`] resource as
/// [`Option`]al borrows, draining the reader and returning early if any is missing
/// (`bevy-traps.md` #1). Param-only (`bevy-traps.md` #7).
pub(in crate::states::load) fn redrive_fields_on_asset_event(
    mut events: MessageReader<AssetEvent<RonAsset<FieldDef>>>,
    asset_server: Option<Res<AssetServer>>,
    folder_handle: Option<Res<ActiveFieldsFolderHandle>>,
    folders: Option<Res<Assets<LoadedFolder>>>,
    field_defs: Option<Res<Assets<RonAsset<FieldDef>>>>,
    registry: Option<ResMut<FieldDefRegistry>>,
) {
    let (
        Some(asset_server),
        Some(folder_handle),
        Some(folders),
        Some(field_defs),
        Some(mut registry),
    ) = (asset_server, folder_handle, folders, field_defs, registry)
    else {
        // Drain the reader so a pre-resolve event does not linger and re-fire once the
        // resources arrive; there is nothing to rebuild yet.
        events.clear();
        return;
    };

    // Rebuild on ANY modified field member — a single rebuild from the latest in-memory defs
    // covers however many member events arrived this frame.
    let modified = events
        .read()
        .any(|event| matches!(event, AssetEvent::Modified { .. }));
    if !modified {
        return;
    }

    let Some(rebuilt) = build_field_registry(&asset_server, &folders, &field_defs, &folder_handle)
    else {
        // A member def is mid-reload (not yet back in the collection) — leave the existing
        // registry until it settles; the next event rebuilds.
        return;
    };
    *registry = rebuilt;
    info!(
        "field hot-reload: rebuilt FieldDefRegistry from `assets/content/fields/` ({} field types)",
        registry.len(),
    );
}

/// The field KEY for a loaded field file's stem — the stem with the dedicated `.field` infix
/// stripped (GTW-545).
///
/// A field file is `<key>.field.ron`; Bevy's `file_stem()` yields `<key>.field`, so the KEY
/// (the [`FieldKey`] a situation references) is that stem minus a trailing `.field`. A stem
/// without the infix is returned unchanged (defensive — keeps a mis-named file's key its plain
/// stem).
fn field_key_from_stem(stem: &str) -> String {
    stem.strip_suffix(".field").unwrap_or(stem).to_owned()
}
