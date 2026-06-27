//! The editor's `Load`-scoped asset handles.
//!
//! Mirrors `gdtf_app`'s `LoadHandles` shape, trimmed to exactly what the editor shell
//! needs: the theme RON, plus the weapon / armor / theme-catalog folders. Each is a
//! named newtype over its Bevy handle (no-bare-types rule 5: private inner, derived
//! [`Deref`], a `new` constructor) so a folder handle can never be mixed up with the
//! theme handle. The resource lives only during [`EditorState::Load`](crate::EditorState)
//! — it is removed `OnExit(Load)` once the registries are built (the handles are no
//! longer needed; the registries hold their data BY VALUE).

use bevy::{
    asset::{Handle, LoadedFolder},
    prelude::{Deref, Resource},
};
use gdtf_assets::RonAsset;
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

/// Typed handle to the loaded `content/weapons/` folder — every member is a
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

/// Typed handle to the loaded `content/themes/` folder — every member is a
/// `RonAsset<ThemeSpec>` the poll/resolve pass builds the
/// [`ThemeCatalogRegistry`](gdtf_battle_sim::level::ThemeCatalogRegistry) from (the
/// GTW-409 theme tile catalog).
#[derive(Resource, Deref, Clone, Debug)]
pub(crate) struct EditorThemesFolderHandle(Handle<LoadedFolder>);

impl EditorThemesFolderHandle {
    /// Wrap the themes-folder handle the [`AssetServer`](bevy::asset::AssetServer) returns.
    #[must_use]
    pub(crate) const fn new(handle: Handle<LoadedFolder>) -> Self {
        Self(handle)
    }
}

/// The editor's `Load`-scoped asset handles, inserted by the kick-off system and read by
/// the poll/resolve system. Removed `OnExit(EditorState::Load)`.
#[derive(Resource)]
pub(crate) struct EditorLoadHandles {
    /// The theme-RON handle (resolves to [`GdtfTheme`](gdtf_ui::theme::GdtfTheme)).
    pub(crate) theme:   EditorThemeHandle,
    /// The weapons-folder handle (resolves to the
    /// [`WeaponRegistry`](gdtf_battle_sim::weapon::WeaponRegistry)).
    pub(crate) weapons: EditorWeaponsFolderHandle,
    /// The armor-folder handle (resolves to the
    /// [`ArmorRegistry`](gdtf_battle_sim::armor::ArmorRegistry)).
    pub(crate) armor:   EditorArmorFolderHandle,
    /// The themes-folder handle (resolves to the
    /// [`ThemeCatalogRegistry`](gdtf_battle_sim::level::ThemeCatalogRegistry)).
    pub(crate) themes:  EditorThemesFolderHandle,
}
