//! GTW-533: the editor's LIVE folder-content hot-reload handlers.
//!
//! The editor is a SECOND in-app asset host (a separate binary from the game), and the
//! contract requires assets to hot-reload in-app "game AND editor with NO restart". The
//! game side reloads its content through per-type `redrive_*_on_asset_event` systems in
//! `gdtf_app`; those are private to that crate, so the editor mirrors the SAME pattern here
//! over the editor's OWN registries — reusing the shared Bevy `file_watcher` mechanism (NO
//! second mechanism), not duplicating it.
//!
//! These four handlers cover the editor's FOLDER-loaded content — the ranged
//! [`WeaponRegistry`], the [`ArmorRegistry`], and the UUID-keyed [`TerrainDefRegistry`] /
//! [`UuidThemeRegistry`]. Each rebuilds its registry in place from the PERSISTENT
//! [`EditorLoadHandles`] via the SAME `build_*_registry` helper the one-time
//! [`resolve`](crate::load::resolve) pass uses, so a live edit yields the same registry a
//! restart would. The editor's other two hot-reloadable assets — the [`GdtfTheme`] and the
//! presenter [`TileRoles`] table — reuse the presenter/ui crates' OWN published redrive
//! systems verbatim (see [`register_load`](crate::load::register_load)); there is no editor
//! copy of those.
//!
//! **Why [`EditorLoadHandles`] persists.** Unlike the game's `LoadHandles` (dropped
//! `OnExit(Load)`), the editor's `register_load` registers NO `OnExit(EditorState::Load)`
//! cleanup, so [`EditorLoadHandles`] lives for the whole session. Holding its folder
//! handles keeps every member asset loaded for the file-watcher and lets these handlers
//! re-enumerate the folder members on a hot edit — the editor analogue of the game's
//! persistent `Active*FolderHandle` resources.
//!
//! Each handler self-guards on its [`Option`]al borrows (`bevy-traps.md` #1) and reads a
//! [`MessageReader`] (asset events are MESSAGES in Bevy 0.19 — `bevy-traps.md` #4), so it
//! is a harmless no-op until the load chain has resolved. Registered in an UNGATED `Update`
//! (they must fire AFTER `Load` exits, once the editor is `Editing`), guarded only by the
//! `AssetServer`-present check in `register_load` (the `Messages<AssetEvent<…>>` buffers
//! these readers need are registered by `init_ron_asset`).

use bevy::{
    asset::{AssetEvent, AssetServer, Assets, LoadedFolder},
    prelude::{MessageReader, Res, ResMut, info},
};
use gdtf_assets::RonAsset;
use gdtf_battle_sim::{
    armor::{ArmorRegistry, ArmorSpec},
    level::{UuidThemeDef, UuidThemeRegistry},
    terrain::def::{TerrainDef, TerrainDefRegistry},
    weapon::{WeaponRegistry, WeaponSpec},
};

use crate::load::{
    handles::EditorLoadHandles,
    resolve::{
        build_armor_registry, build_terrain_def_registry, build_theme_def_registry,
        build_weapon_registry,
    },
};

/// `Update`: rebuild the editor's [`WeaponRegistry`] in place on a matching
/// [`AssetEvent::Modified`] for any member `content/weapons/ranged/*.weapon.ron` — the
/// GTW-533 editor mirror of the game's `redrive_weapons_on_asset_event`.
///
/// Reacts to ANY `AssetEvent<RonAsset<WeaponSpec>>::Modified` and rebuilds the whole
/// registry from the persistent [`EditorLoadHandles`]' weapons-folder handle via
/// [`build_weapon_registry`] — the SAME builder the one-time resolve uses. Overwriting via
/// [`ResMut`] marks the registry changed. Guarded on [`Option`]al borrows so it is a
/// harmless no-op pre-`Load` (`bevy-traps.md` #1).
pub(crate) fn redrive_weapons_on_asset_event(
    mut events: MessageReader<AssetEvent<RonAsset<WeaponSpec>>>,
    asset_server: Option<Res<AssetServer>>,
    handles: Option<Res<EditorLoadHandles>>,
    folders: Option<Res<Assets<LoadedFolder>>>,
    weapon_specs: Option<Res<Assets<RonAsset<WeaponSpec>>>>,
    registry: Option<ResMut<WeaponRegistry>>,
) {
    let (Some(asset_server), Some(handles), Some(folders), Some(weapon_specs), Some(mut registry)) =
        (asset_server, handles, folders, weapon_specs, registry)
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

    let Some(rebuilt) =
        build_weapon_registry(&asset_server, &folders, &weapon_specs, &handles.weapons)
    else {
        // A member spec is mid-reload — leave the existing registry until it settles.
        return;
    };
    *registry = rebuilt;
    info!(
        "editor weapon hot-reload: rebuilt WeaponRegistry from `content/weapons/ranged/` ({} \
         weapons)",
        registry.len(),
    );
}

/// `Update`: rebuild the editor's [`ArmorRegistry`] in place on a matching
/// [`AssetEvent::Modified`] for any member `content/armor/*.armor.ron` — the GTW-533 editor
/// mirror of the game's `redrive_armor_on_asset_event`.
///
/// Reacts to ANY `AssetEvent<RonAsset<ArmorSpec>>::Modified` and rebuilds the whole registry
/// via [`build_armor_registry`]. Guarded on [`Option`]al borrows (`bevy-traps.md` #1).
pub(crate) fn redrive_armor_on_asset_event(
    mut events: MessageReader<AssetEvent<RonAsset<ArmorSpec>>>,
    asset_server: Option<Res<AssetServer>>,
    handles: Option<Res<EditorLoadHandles>>,
    folders: Option<Res<Assets<LoadedFolder>>>,
    armor_specs: Option<Res<Assets<RonAsset<ArmorSpec>>>>,
    registry: Option<ResMut<ArmorRegistry>>,
) {
    let (Some(asset_server), Some(handles), Some(folders), Some(armor_specs), Some(mut registry)) =
        (asset_server, handles, folders, armor_specs, registry)
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

    let Some(rebuilt) = build_armor_registry(&asset_server, &folders, &armor_specs, &handles.armor)
    else {
        return;
    };
    *registry = rebuilt;
    info!(
        "editor armor hot-reload: rebuilt ArmorRegistry from `content/armor/` ({} pieces)",
        registry.len(),
    );
}

/// `Update`: rebuild the editor's UUID-keyed [`TerrainDefRegistry`] in place on a matching
/// [`AssetEvent::Modified`] for any member `terrain/**/*.terrain_def.ron` — the GTW-533
/// editor mirror of the game's `redrive_terrain_defs_on_asset_event`.
///
/// Reacts to ANY `AssetEvent<RonAsset<TerrainDef>>::Modified` and rebuilds the whole registry
/// from the persistent per-theme terrain-model folder handle via [`build_terrain_def_registry`].
/// Guarded on [`Option`]al borrows (`bevy-traps.md` #1).
pub(crate) fn redrive_terrain_defs_on_asset_event(
    mut events: MessageReader<AssetEvent<RonAsset<TerrainDef>>>,
    handles: Option<Res<EditorLoadHandles>>,
    folders: Option<Res<Assets<LoadedFolder>>>,
    terrain_defs: Option<Res<Assets<RonAsset<TerrainDef>>>>,
    registry: Option<ResMut<TerrainDefRegistry>>,
) {
    let (Some(handles), Some(folders), Some(terrain_defs), Some(mut registry)) =
        (handles, folders, terrain_defs, registry)
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

    let Some(rebuilt) = build_terrain_def_registry(&folders, &terrain_defs, &handles.terrain_model)
    else {
        return;
    };
    *registry = rebuilt;
    info!(
        "editor terrain-def hot-reload: rebuilt TerrainDefRegistry from `terrain/` ({} defs)",
        registry.len(),
    );
}

/// `Update`: rebuild the editor's UUID-keyed [`UuidThemeRegistry`] in place on a matching
/// [`AssetEvent::Modified`] for any member `terrain/**/*.terrain_theme.ron` — the GTW-533
/// editor mirror of the game's `redrive_theme_defs_on_asset_event`, the theme companion of
/// [`redrive_terrain_defs_on_asset_event`].
///
/// Reacts to ANY `AssetEvent<RonAsset<UuidThemeDef>>::Modified` and rebuilds the whole
/// registry via [`build_theme_def_registry`]. Guarded on [`Option`]al borrows
/// (`bevy-traps.md` #1).
pub(crate) fn redrive_theme_defs_on_asset_event(
    mut events: MessageReader<AssetEvent<RonAsset<UuidThemeDef>>>,
    handles: Option<Res<EditorLoadHandles>>,
    folders: Option<Res<Assets<LoadedFolder>>>,
    theme_defs: Option<Res<Assets<RonAsset<UuidThemeDef>>>>,
    registry: Option<ResMut<UuidThemeRegistry>>,
) {
    let (Some(handles), Some(folders), Some(theme_defs), Some(mut registry)) =
        (handles, folders, theme_defs, registry)
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

    let Some(rebuilt) = build_theme_def_registry(&folders, &theme_defs, &handles.terrain_model)
    else {
        return;
    };
    *registry = rebuilt;
    info!(
        "editor theme-def hot-reload: rebuilt UuidThemeRegistry from `terrain/` ({} themes)",
        registry.len(),
    );
}

#[cfg(test)]
mod test;
