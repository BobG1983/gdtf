//! The editor's `Load`-scoped asset handles.
//!
//! Mirrors `gdtf_app`'s `LoadHandles` shape, trimmed to exactly what the editor shell
//! needs: the theme RON, the weapon / armor folders, the NEW UUID-keyed per-theme
//! `terrain/` folder (GTW-487), and the presenter's tile-role RON (GTW-495 — the per-def
//! graphic resolution the editor mirrors). Each is a named newtype over its Bevy handle
//! (no-bare-types rule 5: private inner, derived [`Deref`], a `new` constructor). The
//! GTW-533: the resource PERSISTS for the whole session — `register_load` registers NO
//! `OnExit(EditorState::Load)` cleanup — so the editor's LIVE hot-reload handlers
//! ([`redrive`](crate::load::redrive)) can re-enumerate the folder members on a hot `.ron`
//! edit, and holding the folder handles keeps every member asset loaded for the file-watcher
//! (the editor analogue of the game's persistent `Active*FolderHandle` resources). The built
//! registries hold their data BY VALUE, so they too survive independently.

use bevy::{
    asset::{Handle, LoadedFolder},
    prelude::{Deref, Resource},
};
use gdtf_assets::RonAsset;
use gdtf_battle_presenter::TileRoles;
use gdtf_ui::theme::GdtfThemeSpec;

/// Typed handle to the loose theme RON (`core_tuning/ui_theme.tuning.ron`), loaded as a
/// `RonAsset<GdtfThemeSpec>` through the generic loader — the editor's themed regions are
/// painted from the resolved [`GdtfTheme`](gdtf_ui::theme::GdtfTheme).
#[derive(Resource, Deref, Clone, Debug)]
pub(crate) struct EditorThemeHandle(Handle<RonAsset<GdtfThemeSpec>>);

impl EditorThemeHandle {
    /// Wrap the theme-RON handle the [`AssetServer`](bevy::asset::AssetServer) returns.
    #[must_use]
    pub(crate) const fn new(handle: Handle<RonAsset<GdtfThemeSpec>>) -> Self {
        Self(handle)
    }
}

/// Typed handle to the loaded `content/weapons/ranged/` folder — every member is a
/// `RonAsset<WeaponSpec>` the poll/resolve pass builds the
/// [`WeaponRegistry`](gdtf_battle_sim::weapon::WeaponRegistry) from.
#[derive(Resource, Deref, Clone, Debug)]
pub(crate) struct EditorWeaponsFolderHandle(Handle<LoadedFolder>);

impl EditorWeaponsFolderHandle {
    /// Wrap the weapons-folder handle the [`AssetServer`](bevy::asset::AssetServer) returns.
    #[must_use]
    pub(crate) const fn new(handle: Handle<LoadedFolder>) -> Self {
        Self(handle)
    }
}

/// Typed handle to the loaded `content/armor/` folder — every member is a
/// `RonAsset<ArmorSpec>` the poll/resolve pass builds the
/// [`ArmorRegistry`](gdtf_battle_sim::armor::ArmorRegistry) from.
#[derive(Resource, Deref, Clone, Debug)]
pub(crate) struct EditorArmorFolderHandle(Handle<LoadedFolder>);

impl EditorArmorFolderHandle {
    /// Wrap the armor-folder handle the [`AssetServer`](bevy::asset::AssetServer) returns.
    #[must_use]
    pub(crate) const fn new(handle: Handle<LoadedFolder>) -> Self {
        Self(handle)
    }
}

/// Typed handle to the loaded NEW per-theme `terrain/` folder (GTW-487) — its members are
/// `RonAsset<TerrainDef>` (`*.terrain_def.ron`) + `RonAsset<UuidThemeDef>`
/// (`*.terrain_theme.ron`) the poll/resolve pass builds the UUID-keyed
/// [`TerrainDefRegistry`](gdtf_battle_sim::terrain::def::TerrainDefRegistry) +
/// [`UuidThemeRegistry`](gdtf_battle_sim::level::UuidThemeRegistry) from. ONE recursive
/// `load_folder` of `terrain/` feeds both — the UUID-keyed models are the editor's SOLE
/// terrain/theme source after GTW-495 (the legacy `content/themes` catalog is retired).
#[derive(Resource, Deref, Clone, Debug)]
pub(crate) struct EditorTerrainModelFolderHandle(Handle<LoadedFolder>);

impl EditorTerrainModelFolderHandle {
    /// Wrap the new per-theme terrain-model-folder handle the
    /// [`AssetServer`](bevy::asset::AssetServer) returns.
    #[must_use]
    pub(crate) const fn new(handle: Handle<LoadedFolder>) -> Self {
        Self(handle)
    }
}

/// Typed handle to the loose tile-role RON (`sprites/tile_roles.spritedef.ron`), loaded as a
/// `RonAsset<TileRoles>` (GTW-495). The editor resolves a [`TerrainDef`](gdtf_battle_sim::terrain::def::TerrainDef)'s
/// `presenter_kind.graphic_name` to a terrain atlas index THROUGH this presenter-owned
/// [`TileRoles`] table (`TileRoles::index_for_key`) — the SAME role vocabulary the battlescape
/// presenter resolves graphics by (GTW-493), so the editor's palette / canvas sprites match the
/// game's.
#[derive(Resource, Deref, Clone, Debug)]
pub(crate) struct EditorTileRolesHandle(Handle<RonAsset<TileRoles>>);

impl EditorTileRolesHandle {
    /// Wrap the tile-role RON handle the [`AssetServer`](bevy::asset::AssetServer) returns.
    #[must_use]
    pub(crate) const fn new(handle: Handle<RonAsset<TileRoles>>) -> Self {
        Self(handle)
    }
}

/// The editor's asset handles, inserted by the kick-off system and read by the poll/resolve
/// system. GTW-533: PERSISTS for the whole session (no `OnExit(Load)` cleanup is registered)
/// so the [`redrive`](crate::load::redrive) hot-reload handlers can re-enumerate folder
/// members on a live `.ron` edit, and holding the folder handles keeps every member asset
/// loaded for the file-watcher.
#[derive(Resource)]
pub(crate) struct EditorLoadHandles {
    /// The theme-RON handle (resolves to [`GdtfTheme`](gdtf_ui::theme::GdtfTheme)).
    pub(crate) theme:         EditorThemeHandle,
    /// The weapons-folder handle (resolves to the
    /// [`WeaponRegistry`](gdtf_battle_sim::weapon::WeaponRegistry)).
    pub(crate) weapons:       EditorWeaponsFolderHandle,
    /// The armor-folder handle (resolves to the
    /// [`ArmorRegistry`](gdtf_battle_sim::armor::ArmorRegistry)).
    pub(crate) armor:         EditorArmorFolderHandle,
    /// The NEW per-theme `terrain/` folder handle (GTW-487 — resolves to the UUID-keyed
    /// [`TerrainDefRegistry`](gdtf_battle_sim::terrain::def::TerrainDefRegistry) +
    /// [`UuidThemeRegistry`](gdtf_battle_sim::level::UuidThemeRegistry)).
    pub(crate) terrain_model: EditorTerrainModelFolderHandle,
    /// The tile-role RON handle (GTW-495 — resolves to the presenter's
    /// [`TileRoles`](gdtf_battle_presenter::TileRoles) table the editor resolves per-def
    /// graphics through).
    pub(crate) tile_roles:    EditorTileRolesHandle,
}
