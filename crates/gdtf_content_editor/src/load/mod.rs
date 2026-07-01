//! The editor's slim `Load` pass: kick off the asset loads, poll/resolve them into the
//! theme + registries + tile-role table, then transition to
//! [`Editing`](crate::EditorState::Editing).
//!
//! Wiring-only module. The kick-off / resolve / transition logic lives in focused
//! submodules; this file registers them on the [`EditorState::Load`](crate::EditorState)
//! schedule and owns the RON-loader registration (so `asset_server.load::<RonAsset<T>>`
//! and `load_folder` of the dedicated compound extensions are dispatched correctly).

mod handles;
mod kick_off;
mod redrive;
mod resolve;
mod transition;

use bevy::{asset::AssetServer, prelude::*};
use gdtf_assets::RonAssetAppExt;
use gdtf_battle_presenter::{TileRoles, redrive_tile_roles_on_asset_event};
use gdtf_battle_sim::{
    armor::ArmorSpec, level::UuidThemeDef, terrain::def::TerrainDef, weapon::WeaponSpec,
};
use gdtf_ui::{redrive_theme_on_asset_event, theme::GdtfThemeSpec};
pub(crate) use transition::transition_to_editing;

use crate::{
    EditorState,
    load::{kick_off::kick_off_editor_loads, resolve::poll_and_resolve_editor},
};

/// Registers the editor's `Load` asset pass onto `app`.
///
/// Mirrors the game's `LoadScenePlugin`, trimmed to the editor's loads:
///
/// - Registers the generic RON loader for each editor asset type behind an
///   `AssetServer`-present guard (`bevy-traps.md` #1): the theme spec + the tile-role table
///   (each loaded by path), the weapon / armor specs (each via `load_folder` of its OWN
///   dedicated compound extension), and the GTW-487 UUID-keyed terrain-def / theme-def types
///   (each via its OWN dedicated `terrain_def.ron` / `terrain_theme.ron` extension so the
///   per-theme `terrain/` folder dispatch is unambiguous — the game's loader scheme).
/// - `OnEnter(Load)`: kick off the loads.
/// - `Update` (while `Load` and any target resource is still absent): poll + resolve.
/// - `Update` (while `Load` and all resources exist): transition to `Editing`.
/// - `Update` (UNGATED — these fire AFTER `Load` exits, once `Editing`): the GTW-533 LIVE
///   hot-reload handlers, so a live `.ron` edit refreshes the editor's resolved resources
///   with NO restart — the editor half of the "hot-reload in-app (game AND editor)" contract.
///   All SIX editor-hosted asset types are covered through the SAME shared Bevy `file_watcher`
///   mechanism (NO second mechanism): the [`GdtfTheme`](gdtf_ui::theme::GdtfTheme) reuses
///   `gdtf_ui`'s published [`redrive_theme_on_asset_event`] verbatim (the editor's
///   `resolve_theme` already inserts the [`ActiveThemeHandle`](gdtf_ui::theme::ActiveThemeHandle)
///   it filters on); the presenter [`TileRoles`] table reuses `gdtf_battle_presenter`'s
///   published [`redrive_tile_roles_on_asset_event`] verbatim (the editor's `resolve_tile_roles`
///   now inserts the presenter's [`TileRolesHandle`](gdtf_battle_presenter::TileRolesHandle)); and
///   the four FOLDER registries (ranged weapons, armor, the UUID-keyed terrain / theme defs)
///   reuse the editor's own [`redrive`] handlers, which rebuild from the persistent
///   [`EditorLoadHandles`](crate::load::handles::EditorLoadHandles) via the SAME `build_*_registry`
///   helpers the one-time resolve uses. Registered inside the `AssetServer`-present guard: the
///   `Messages<AssetEvent<…>>` buffers these `MessageReader`s need are registered by
///   `init_ron_asset` (`bevy-traps.md` #4).
pub(crate) fn register_load(app: &mut App) {
    if app.world().get_resource::<AssetServer>().is_some() {
        app.init_ron_asset::<GdtfThemeSpec>();
        app.init_ron_asset_with_extensions::<WeaponSpec>(vec!["weapon.ron"]);
        app.init_ron_asset_with_extensions::<ArmorSpec>(vec!["armor.ron"]);
        // GTW-487: the NEW UUID-keyed terrain + theme models, each via its OWN dedicated
        // compound extension so the per-theme `terrain/` folder dispatch is unambiguous.
        app.init_ron_asset_with_extensions::<TerrainDef>(vec!["terrain_def.ron"]);
        app.init_ron_asset_with_extensions::<UuidThemeDef>(vec!["terrain_theme.ron"]);
        // GTW-495: the presenter's tile-role table (the per-def graphic resolution seam). Its
        // `.spritedef.ron` extension is registered by the generic `.ron` loader; the editor does
        // not wire the presenter's render runtime, only resolves graphics through this table.
        app.init_ron_asset::<TileRoles>();

        // GTW-533: the editor's LIVE hot-reload — the editor half of the "game AND editor,
        // NO restart" contract. All SIX editor-hosted asset types reload through the SAME
        // shared file_watcher mechanism (NO second mechanism). Ungated `Update` so they fire
        // once the editor is `Editing`; each self-guards on its `Option`al borrows.
        app.add_systems(
            Update,
            (
                // Reused verbatim from gdtf_ui / gdtf_battle_presenter (no editor copy).
                redrive_theme_on_asset_event,
                redrive_tile_roles_on_asset_event,
                // The editor's own folder-registry redrives (mirror the game's per-type ones).
                redrive::redrive_weapons_on_asset_event,
                redrive::redrive_armor_on_asset_event,
                redrive::redrive_terrain_defs_on_asset_event,
                redrive::redrive_theme_defs_on_asset_event,
            ),
        );
    }

    app.add_systems(OnEnter(EditorState::Load), kick_off_editor_loads);
    app.add_systems(
        Update,
        (poll_and_resolve_editor, transition_to_editing)
            .chain()
            .run_if(in_state(EditorState::Load)),
    );
}
