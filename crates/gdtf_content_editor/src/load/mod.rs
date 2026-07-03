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
use gdtf_assets::{RonAssetAppExt, redrive_hot_ron_resource};
use gdtf_battle_presenter::{TileRoles, tile_roles_hot_ron_chain};
use gdtf_battle_sim::{
    armor::ArmorSpec, level::UuidThemeDef, terrain::def::TerrainDef, weapon::WeaponSpec,
};
use gdtf_ui::{
    theme::{GdtfTheme, GdtfThemeSpec},
    theme_hot_ron_chain,
};
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
///   mechanism (NO second mechanism): the [`GdtfTheme`](gdtf_ui::theme::GdtfTheme) and the
///   presenter [`TileRoles`] table each reuse the GTW-564 GENERIC hot-RON redrive
///   ([`redrive_hot_ron_resource`]) with the chain owner's exported config
///   ([`theme_hot_ron_chain`] / [`tile_roles_hot_ron_chain`]) — the editor's `resolve_theme`
///   / `resolve_tile_roles` insert the generic
///   [`HotRonHandle`](gdtf_assets::HotRonHandle) the redrive filters on; and
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
        //
        // GTW-564: the theme + tile-role halves are the GENERIC hot-RON redrive, run with
        // the chain owners' exported configs (inserted here — the editor registers only
        // the REDRIVE half of each chain; its own bespoke Load pass above owns the
        // kick-off/resolve halves and stores the generic handles the redrives filter on).
        app.insert_resource(theme_hot_ron_chain());
        app.insert_resource(tile_roles_hot_ron_chain());
        app.add_systems(
            Update,
            (
                // The one generic drain body, reused from gdtf_assets (no editor copy).
                redrive_hot_ron_resource::<GdtfThemeSpec, GdtfTheme>,
                redrive_hot_ron_resource::<TileRoles, TileRoles>,
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
