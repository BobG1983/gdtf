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
mod resolve;
mod transition;

use bevy::{asset::AssetServer, prelude::*};
use gdtf_assets::RonAssetAppExt;
use gdtf_battle_presenter::TileRoles;
use gdtf_battle_sim::{
    armor::ArmorSpec, level::UuidThemeDef, terrain::def::TerrainDef, weapon::WeaponSpec,
};
use gdtf_ui::theme::GdtfThemeSpec;
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
    }

    app.add_systems(OnEnter(EditorState::Load), kick_off_editor_loads);
    app.add_systems(
        Update,
        (poll_and_resolve_editor, transition_to_editing)
            .chain()
            .run_if(in_state(EditorState::Load)),
    );
}
