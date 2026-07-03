//! `Update` (during [`EditorState::Load`](crate::EditorState)): poll the in-flight loads
//! and insert the resolved resources.
//!
//! A slim mirror of `gdtf_app`'s `poll_and_resolve`, trimmed to the editor's loads: the
//! [`GdtfTheme`], the legacy [`WeaponRegistry`] / [`ArmorRegistry`], the (GTW-487) NEW
//! UUID-keyed [`TerrainDefRegistry`](gdtf_battle_sim::terrain::def::TerrainDefRegistry) +
//! [`UuidThemeRegistry`](gdtf_battle_sim::level::UuidThemeRegistry) built from the per-theme
//! `content/terrain/` folder, and the (GTW-495) presenter [`TileRoles`] table. Each branch re-gates on
//! its OWN resource's absence so none starves another (`bevy-traps.md` #3), and EVERY branch is
//! fail-safe: a failed asset falls back to a const default (the ADR-0003 error-path safety-net)
//! so the editor never hangs in `Load`. Once all the resources exist, the plugin's transition
//! leaves `Load` for [`Editing`](crate::EditorState::Editing).

use core::any::TypeId;

use bevy::{
    asset::{AssetServer, Assets, Handle, LoadState, LoadedFolder, RecursiveDependencyLoadState},
    ecs::system::SystemParam,
    prelude::*,
};
use gdtf_assets::{HotRonHandle, RonAsset};
use gdtf_battle_presenter::TileRoles;
use gdtf_battle_sim::{
    armor::{ArmorName, ArmorRegistry, ArmorSpec},
    level::{UuidThemeDef, UuidThemeRegistry},
    terrain::def::{TerrainDef, TerrainDefRegistry},
    weapon::{WeaponName, WeaponRegistry, WeaponSpec},
};
use gdtf_ui::{
    resolve_theme_spec,
    theme::{GdtfTheme, GdtfThemeSpec, default_theme},
};

use crate::load::handles::EditorLoadHandles;

/// The loaded RON asset collections [`poll_and_resolve_editor`] reads, bundled into one
/// [`SystemParam`] so the system's parameter list stays under clippy's argument-count gate
/// (the game's `LoadAssetCollections` grouping precedent).
///
/// Each is `Option<Res<…>>` because a headless app without the asset stack has no
/// `AssetServer` (and so no `Assets<…>` collections); the system early-returns when any is
/// absent, so it never panics on a missing collection (`bevy-traps.md` #1).
#[derive(SystemParam)]
pub(crate) struct EditorLoadCollections<'w> {
    /// The loaded theme-spec RON collection (`core_tuning/ui_theme.tuning.ron`).
    theme_specs:  Option<Res<'w, Assets<RonAsset<GdtfThemeSpec>>>>,
    /// The loaded `LoadedFolder` collection — used to read each content folder's members.
    folders:      Option<Res<'w, Assets<LoadedFolder>>>,
    /// The loaded per-weapon RON collection (`content/weapons/ranged/*.weapon.ron`).
    weapon_specs: Option<Res<'w, Assets<RonAsset<WeaponSpec>>>>,
    /// The loaded per-armor RON collection (`content/armor/*.armor.ron`).
    armor_specs:  Option<Res<'w, Assets<RonAsset<ArmorSpec>>>>,
    /// The loaded NEW per-theme terrain-def RON collection
    /// (`content/terrain/<theme>/*.terrain_def.ron`, GTW-487).
    terrain_defs: Option<Res<'w, Assets<RonAsset<TerrainDef>>>>,
    /// The loaded NEW per-theme theme-def RON collection
    /// (`content/terrain/<theme>/*.terrain_theme.ron`, GTW-487).
    theme_defs:   Option<Res<'w, Assets<RonAsset<UuidThemeDef>>>>,
    /// The loaded tile-role RON collection (`sprites/tile_roles.spritedef.ron`, GTW-495).
    tile_roles:   Option<Res<'w, Assets<RonAsset<TileRoles>>>>,
}

/// The persistent resources [`poll_and_resolve_editor`] resolves, each as an
/// `Option<Res<…>>` presence-probe, bundled into one [`SystemParam`] so the system's
/// parameter list stays under clippy's argument-count gate (the game's `ResolvedResources`
/// grouping precedent). Each branch resolves on its OWN resource's absence so none starves
/// another (`bevy-traps.md` #3).
#[derive(SystemParam)]
pub(crate) struct EditorResolved<'w> {
    /// Whether the resolved [`GdtfTheme`] is already inserted.
    theme:        Option<Res<'w, GdtfTheme>>,
    /// Whether the resolved [`WeaponRegistry`] is already inserted.
    weapons:      Option<Res<'w, WeaponRegistry>>,
    /// Whether the resolved [`ArmorRegistry`] is already inserted.
    armor:        Option<Res<'w, ArmorRegistry>>,
    /// Whether the resolved NEW [`TerrainDefRegistry`] is already inserted (GTW-487).
    terrain_defs: Option<Res<'w, TerrainDefRegistry>>,
    /// Whether the resolved NEW [`UuidThemeRegistry`] is already inserted (GTW-487).
    theme_defs:   Option<Res<'w, UuidThemeRegistry>>,
    /// Whether the resolved [`TileRoles`] table is already inserted (GTW-495).
    tile_roles:   Option<Res<'w, TileRoles>>,
}

/// Polls the editor's in-flight loads and inserts each resolved resource on its OWN
/// absence guard.
///
/// Runs while [`EditorLoadHandles`] exists and ANY of the target resources ([`GdtfTheme`],
/// [`WeaponRegistry`], [`ArmorRegistry`], the GTW-487 [`TerrainDefRegistry`] +
/// [`UuidThemeRegistry`], and the GTW-495 [`TileRoles`]) is still missing. Each branch resolves
/// independently and falls back to a const default / empty registry on a failed load, so a
/// slow/bad asset never strands the editor. Takes its borrows as `Option<…>` so a headless app
/// without the asset stack no-ops rather than panics (`bevy-traps.md` #1).
pub(crate) fn poll_and_resolve_editor(
    mut commands: Commands,
    asset_server: Option<Res<AssetServer>>,
    collections: EditorLoadCollections,
    resolved: EditorResolved,
    handles: Option<Res<EditorLoadHandles>>,
) {
    let (theme_done, weapons_done, armor_done, terrain_defs_done, theme_defs_done, roles_done) = (
        resolved.theme.is_some(),
        resolved.weapons.is_some(),
        resolved.armor.is_some(),
        resolved.terrain_defs.is_some(),
        resolved.theme_defs.is_some(),
        resolved.tile_roles.is_some(),
    );
    let (
        Some(asset_server),
        Some(theme_assets),
        Some(folders),
        Some(weapon_specs),
        Some(armor_specs),
        Some(terrain_defs),
        Some(theme_defs),
        Some(roles_assets),
        Some(handles),
    ) = (
        asset_server,
        collections.theme_specs,
        collections.folders,
        collections.weapon_specs,
        collections.armor_specs,
        collections.terrain_defs,
        collections.theme_defs,
        collections.tile_roles,
        handles,
    )
    else {
        return;
    };

    if !theme_done {
        resolve_theme(&mut commands, &asset_server, &theme_assets, &handles);
    }
    if !weapons_done {
        resolve_weapons(
            &mut commands,
            &asset_server,
            &folders,
            &weapon_specs,
            &handles,
        );
    }
    if !armor_done {
        resolve_armor(
            &mut commands,
            &asset_server,
            &folders,
            &armor_specs,
            &handles,
        );
    }
    // GTW-487: the NEW UUID-keyed terrain + theme registries, built from the per-theme
    // `content/terrain/` folder. EMPTY against un-migrated content is the designed fail-closed state.
    if !terrain_defs_done {
        resolve_terrain_defs(
            &mut commands,
            &asset_server,
            &folders,
            &terrain_defs,
            &handles,
        );
    }
    if !theme_defs_done {
        resolve_theme_defs(
            &mut commands,
            &asset_server,
            &folders,
            &theme_defs,
            &handles,
        );
    }
    // GTW-495: the presenter's tile-role table (the per-def graphic resolution seam).
    if !roles_done {
        resolve_tile_roles(&mut commands, &asset_server, &roles_assets, &handles);
    }
}

/// Resolve the loaded theme RON into a [`GdtfTheme`], or fall back to the const default
/// theme on a failed load — mirrors the game's `poll_and_resolve` theme branch. Inserts
/// the persistent generic [`HotRonHandle`]`<GdtfThemeSpec>` on both paths so a later
/// file-watcher reload can recover (the editor reuses the same GTW-564 theming seam as
/// the game: the generic redrive it registers filters against this handle).
fn resolve_theme(
    commands: &mut Commands,
    asset_server: &AssetServer,
    theme_assets: &Assets<RonAsset<GdtfThemeSpec>>,
    handles: &EditorLoadHandles,
) {
    let theme_state = asset_server.load_state(&*handles.theme);

    if theme_state.is_failed() {
        warn!(
            "GDTF editor Load: theme `core_tuning/ui_theme.tuning.ron` failed to load; falling \
             back to the const default theme",
        );
        commands.insert_resource(default_theme());
        commands.insert_resource(HotRonHandle::new((*handles.theme).clone()));
        return;
    }

    if matches!(theme_state, LoadState::Loaded) {
        let Some(spec) = theme_assets.get(&*handles.theme) else {
            // Loaded-but-not-yet-in-collection: a transient one-frame state; retry next frame.
            return;
        };
        // Resolve through the ONE shared map hook (gdtf_ui::resolve_theme_spec) — each
        // font key loads on demand (idempotent; the editor does not preload a fonts
        // folder like the game), and the generic redrive re-runs the SAME hook on a hot
        // edit.
        let theme: GdtfTheme = resolve_theme_spec(spec, asset_server);
        commands.insert_resource(theme);
        commands.insert_resource(HotRonHandle::new((*handles.theme).clone()));
    }
}

/// Resolve the loaded `content/weapons/ranged/` folder into the name-keyed [`WeaponRegistry`], or
/// fall back to an empty registry on a failed folder — the editor mirror of the game's
/// `resolve_weapons`.
fn resolve_weapons(
    commands: &mut Commands,
    asset_server: &AssetServer,
    folders: &Assets<LoadedFolder>,
    weapon_specs: &Assets<RonAsset<WeaponSpec>>,
    handles: &EditorLoadHandles,
) {
    let folder_state = asset_server.recursive_dependency_load_state(&*handles.weapons);

    if matches!(folder_state, RecursiveDependencyLoadState::Failed(_)) {
        warn!(
            "GDTF editor Load: the `weapons` folder failed to load; inserting an empty \
             WeaponRegistry",
        );
        commands.insert_resource(WeaponRegistry::default());
        return;
    }

    if matches!(folder_state, RecursiveDependencyLoadState::Loaded) {
        let Some(registry) =
            build_weapon_registry(asset_server, folders, weapon_specs, &handles.weapons)
        else {
            // A member spec not yet in its collection: retry next frame (do NOT publish
            // a partial registry).
            return;
        };
        commands.insert_resource(registry);
    }
}

/// Build the name-keyed [`WeaponRegistry`] from a loaded `content/weapons/ranged/`
/// [`LoadedFolder`], or [`None`] if the folder (or any member spec) is not yet in its
/// collection.
///
/// Shared by [`resolve_weapons`] (the one-time `Load`-state build) and
/// [`redrive_weapons_on_asset_event`](crate::load::redrive::redrive_weapons_on_asset_event)
/// (the GTW-533 live rebuild on a hot edit), so both build the registry IDENTICALLY — a
/// live edit yields the same registry a restart would. Mirrors the game's
/// `build_weapon_registry`.
pub(crate) fn build_weapon_registry(
    asset_server: &AssetServer,
    folders: &Assets<LoadedFolder>,
    weapon_specs: &Assets<RonAsset<WeaponSpec>>,
    folder_handle: &Handle<LoadedFolder>,
) -> Option<WeaponRegistry> {
    let folder = folders.get(folder_handle)?;
    let mut registry = WeaponRegistry::default();
    for untyped in &folder.handles {
        let handle = untyped
            .clone()
            .typed_debug_checked::<RonAsset<WeaponSpec>>();
        // A member spec not yet in its collection — bail (do NOT publish a partial registry).
        let spec = weapon_specs.get(&handle)?;
        let Some(key) = asset_server.get_path(untyped.id()).and_then(|path| {
            path.path()
                .file_stem()
                .map(|stem| key_from_stem(&stem.to_string_lossy(), ".weapon"))
        }) else {
            continue;
        };
        registry.insert(WeaponName::new(key), (**spec).clone());
    }
    Some(registry)
}

/// Resolve the loaded `content/armor/` folder into the name-keyed [`ArmorRegistry`], or
/// fall back to an empty registry on a failed folder — the editor mirror of the game's
/// `resolve_armor`.
fn resolve_armor(
    commands: &mut Commands,
    asset_server: &AssetServer,
    folders: &Assets<LoadedFolder>,
    armor_specs: &Assets<RonAsset<ArmorSpec>>,
    handles: &EditorLoadHandles,
) {
    let folder_state = asset_server.recursive_dependency_load_state(&*handles.armor);

    if matches!(folder_state, RecursiveDependencyLoadState::Failed(_)) {
        warn!(
            "GDTF editor Load: the `armor` folder failed to load; inserting an empty \
             ArmorRegistry",
        );
        commands.insert_resource(ArmorRegistry::default());
        return;
    }

    if matches!(folder_state, RecursiveDependencyLoadState::Loaded) {
        let Some(registry) =
            build_armor_registry(asset_server, folders, armor_specs, &handles.armor)
        else {
            return;
        };
        commands.insert_resource(registry);
    }
}

/// Build the name-keyed [`ArmorRegistry`] from a loaded `content/armor/` [`LoadedFolder`],
/// or [`None`] if the folder (or any member spec) is not yet in its collection.
///
/// Shared by [`resolve_armor`] (the one-time `Load`-state build) and
/// [`redrive_armor_on_asset_event`](crate::load::redrive::redrive_armor_on_asset_event)
/// (the GTW-533 live rebuild on a hot edit), so both build the registry IDENTICALLY.
/// Mirrors the game's `build_armor_registry`.
pub(crate) fn build_armor_registry(
    asset_server: &AssetServer,
    folders: &Assets<LoadedFolder>,
    armor_specs: &Assets<RonAsset<ArmorSpec>>,
    folder_handle: &Handle<LoadedFolder>,
) -> Option<ArmorRegistry> {
    let folder = folders.get(folder_handle)?;
    let mut registry = ArmorRegistry::default();
    for untyped in &folder.handles {
        let handle = untyped.clone().typed_debug_checked::<RonAsset<ArmorSpec>>();
        // A member spec not yet in its collection — bail (do NOT publish a partial registry).
        let spec = armor_specs.get(&handle)?;
        let Some(key) = asset_server.get_path(untyped.id()).and_then(|path| {
            path.path()
                .file_stem()
                .map(|stem| key_from_stem(&stem.to_string_lossy(), ".armor"))
        }) else {
            continue;
        };
        registry.insert(ArmorName::new(key), **spec);
    }
    Some(registry)
}

/// Resolve the loaded NEW per-theme `content/terrain/` folder into the UUID-keyed
/// [`TerrainDefRegistry`], or fall back to an empty registry on a failed folder — the editor
/// mirror of the game's `resolve_terrain_defs` (GTW-487). Keyed by each def's OWN
/// [`TerrainUuid`](gdtf_battle_sim::terrain::def::TerrainUuid). A folder member that is NOT a
/// `RonAsset<TerrainDef>` (a theme-def member) is skipped — the recursive `Loaded` gate
/// guarantees every member has landed, so a miss is a non-terrain member, not a pending def.
fn resolve_terrain_defs(
    commands: &mut Commands,
    asset_server: &AssetServer,
    folders: &Assets<LoadedFolder>,
    terrain_defs: &Assets<RonAsset<TerrainDef>>,
    handles: &EditorLoadHandles,
) {
    let folder_state = asset_server.recursive_dependency_load_state(&*handles.terrain_model);

    if matches!(folder_state, RecursiveDependencyLoadState::Failed(_)) {
        warn!(
            "GDTF editor Load: the new per-theme `terrain` folder failed to load; inserting an \
             empty TerrainDefRegistry",
        );
        commands.insert_resource(TerrainDefRegistry::default());
        return;
    }

    if matches!(folder_state, RecursiveDependencyLoadState::Loaded) {
        let Some(registry) =
            build_terrain_def_registry(folders, terrain_defs, &handles.terrain_model)
        else {
            // A terrain-def member mid-load — retry next frame (do NOT publish a partial).
            return;
        };
        commands.insert_resource(registry);
    }
}

/// Build the UUID-keyed [`TerrainDefRegistry`] from a loaded per-theme `content/terrain/`
/// [`LoadedFolder`], or [`None`] if the folder (or any `RonAsset<TerrainDef>` member) is
/// not yet in its collection.
///
/// Shared by [`resolve_terrain_defs`] (the one-time `Load`-state build) and
/// [`redrive_terrain_defs_on_asset_event`](crate::load::redrive::redrive_terrain_defs_on_asset_event)
/// (the GTW-533 live rebuild on a hot edit), so both build the registry IDENTICALLY.
/// Mirrors the game's `build_terrain_def_registry` — the folder is MIXED (it also holds
/// theme members), so each member is FIRST filtered by [`TypeId`] to avoid the debug-assert
/// panic a blind `typed_debug_checked` on a theme member would trip.
pub(crate) fn build_terrain_def_registry(
    folders: &Assets<LoadedFolder>,
    terrain_defs: &Assets<RonAsset<TerrainDef>>,
    folder_handle: &Handle<LoadedFolder>,
) -> Option<TerrainDefRegistry> {
    let folder = folders.get(folder_handle)?;
    let mut registry = TerrainDefRegistry::default();
    for untyped in &folder.handles {
        // Skip non-terrain members (theme defs live in a different collection) — the TypeId
        // filter FIRST avoids the debug-assert panic a blind typed_debug_checked on a theme
        // member would trip (the mixed-folder hazard the game side shares).
        if untyped.type_id() != TypeId::of::<RonAsset<TerrainDef>>() {
            continue;
        }
        let handle = untyped
            .clone()
            .typed_debug_checked::<RonAsset<TerrainDef>>();
        // A terrain-def member mid-load — bail (do NOT publish a partial).
        let def = terrain_defs.get(&handle)?;
        let def = (**def).clone();
        registry.insert(def.key, def);
    }
    Some(registry)
}

/// Resolve the loaded NEW per-theme `content/terrain/` folder into the UUID-keyed
/// [`UuidThemeRegistry`], or fall back to an empty registry on a failed folder — the editor
/// mirror of the game's `resolve_theme_defs` (GTW-487). Keyed by each def's OWN
/// [`ThemeUuid`](gdtf_battle_sim::level::ThemeUuid). A non-theme member (a terrain def) is
/// skipped (no entry in this collection).
fn resolve_theme_defs(
    commands: &mut Commands,
    asset_server: &AssetServer,
    folders: &Assets<LoadedFolder>,
    theme_defs: &Assets<RonAsset<UuidThemeDef>>,
    handles: &EditorLoadHandles,
) {
    let folder_state = asset_server.recursive_dependency_load_state(&*handles.terrain_model);

    if matches!(folder_state, RecursiveDependencyLoadState::Failed(_)) {
        warn!(
            "GDTF editor Load: the new per-theme `terrain` folder failed to load; inserting an \
             empty UuidThemeRegistry",
        );
        commands.insert_resource(UuidThemeRegistry::default());
        return;
    }

    if matches!(folder_state, RecursiveDependencyLoadState::Loaded) {
        let Some(registry) = build_theme_def_registry(folders, theme_defs, &handles.terrain_model)
        else {
            // A theme member mid-load — retry next frame (do NOT publish a partial).
            return;
        };
        commands.insert_resource(registry);
    }
}

/// Build the UUID-keyed [`UuidThemeRegistry`] from a loaded per-theme `content/terrain/`
/// [`LoadedFolder`], or [`None`] if the folder (or any `RonAsset<UuidThemeDef>` member) is
/// not yet in its collection — the theme mirror of [`build_terrain_def_registry`].
///
/// Shared by [`resolve_theme_defs`] (the one-time `Load`-state build) and
/// [`redrive_theme_defs_on_asset_event`](crate::load::redrive::redrive_theme_defs_on_asset_event)
/// (the GTW-533 live rebuild on a hot edit), so both build the registry IDENTICALLY.
pub(crate) fn build_theme_def_registry(
    folders: &Assets<LoadedFolder>,
    theme_defs: &Assets<RonAsset<UuidThemeDef>>,
    folder_handle: &Handle<LoadedFolder>,
) -> Option<UuidThemeRegistry> {
    let folder = folders.get(folder_handle)?;
    let mut registry = UuidThemeRegistry::default();
    for untyped in &folder.handles {
        // Skip non-theme members (terrain defs live in a different collection) — TypeId
        // filter FIRST to avoid the debug-assert panic on a wrong-type typed_debug_checked.
        if untyped.type_id() != TypeId::of::<RonAsset<UuidThemeDef>>() {
            continue;
        }
        let handle = untyped
            .clone()
            .typed_debug_checked::<RonAsset<UuidThemeDef>>();
        // A theme member mid-load — bail (do NOT publish a partial).
        let def = theme_defs.get(&handle)?;
        let def = (**def).clone();
        registry.insert(def.key, def);
    }
    Some(registry)
}

/// Resolve the loaded tile-role RON into the presenter's [`TileRoles`] table, or fall back to
/// the presenter's const default table on a failed load (GTW-495). The editor resolves a
/// terrain def's `presenter_kind.graphic_name` to an atlas index through this table
/// ([`TileRoles::index_for_key`]), so its palette / canvas sprites match the battlescape.
///
/// GTW-533: on success it also inserts the PERSISTENT generic
/// [`HotRonHandle`]`<TileRoles>` (the SAME generic handle the presenter's GTW-564 chain
/// stores), so the generic hot-RON redrive — REUSED in the editor's `Update` with the
/// presenter's exported chain config (no second mechanism) — can filter incoming
/// `AssetEvent` ids against it and re-resolve the resident `TileRoles` on a live
/// `sprites/tile_roles.spritedef.ron` edit. The handle persists past `Load` (nothing removes
/// it) and keeps the tile-role asset loaded for the file-watcher.
fn resolve_tile_roles(
    commands: &mut Commands,
    asset_server: &AssetServer,
    roles_assets: &Assets<RonAsset<TileRoles>>,
    handles: &EditorLoadHandles,
) {
    let state = asset_server.load_state(&*handles.tile_roles);

    if state.is_failed() {
        warn!(
            "GDTF editor Load: `sprites/tile_roles.spritedef.ron` failed to load; inserting the \
             default TileRoles table",
        );
        commands.insert_resource(default_tile_roles());
        // Insert the persistent handle even on the failure path so a later file-watcher
        // reload of a fixed `tile_roles.ron` can still re-resolve (mirrors the theme
        // failure-path handle insert).
        commands.insert_resource(HotRonHandle::new((*handles.tile_roles).clone()));
        return;
    }

    if matches!(state, LoadState::Loaded) {
        let Some(loaded) = roles_assets.get(&*handles.tile_roles) else {
            // Loaded-but-not-yet-in-collection: a transient one-frame state; retry next frame.
            return;
        };
        commands.insert_resource((**loaded).clone());
        commands.insert_resource(HotRonHandle::new((*handles.tile_roles).clone()));
    }
}

/// The fallback [`TileRoles`] table — every role at index `0` — used only when the
/// `tile_roles.ron` fails to load, so a bad asset never strands the editor in `Load`. A
/// degraded but non-panicking table (every terrain then draws the sheet's first tile).
const fn default_tile_roles() -> TileRoles {
    let zero = gdtf_battle_presenter::TileIndex::new(0);
    TileRoles {
        floor:                zero,
        floor_alt_panel:      zero,
        wall:                 zero,
        wall_ew:              zero,
        cover:                zero,
        emplacement:          zero,
        emplacement_occupied: zero,
        slab:                 zero,
        rubble:               zero,
        slab_destroyed:       zero,
        door:                 zero,
        stair_up:             zero,
        stair_down:           zero,
        ladder:               zero,
        door_ns:              zero,
        door_ew:              zero,
        stair_ns_up:          zero,
        stair_ns_down:        zero,
        stair_ew_up:          zero,
        stair_ew_down:        zero,
    }
}

/// Strip a dedicated compound-extension infix (e.g. `.weapon`, `.armor`) off an asset
/// file STEM to recover its registry KEY: `stub_pistol.weapon.ron`'s file stem is
/// `stub_pistol.weapon`, whose key is `stub_pistol`. A stem without the infix is returned
/// unchanged (defensive). Mirrors the game's `weapon_key_from_stem` / `armor_key_from_stem`.
fn key_from_stem(stem: &str, infix: &str) -> String {
    stem.strip_suffix(infix).unwrap_or(stem).to_owned()
}
