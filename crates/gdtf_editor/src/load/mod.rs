//! The editor's slim `Load` pass: kick off the asset loads, poll/resolve them into the
//! theme + registries, then transition to [`Editing`](crate::EditorState::Editing).
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
use gdtf_battle_sim::{armor::ArmorSpec, level::ThemeSpec, weapon::WeaponSpec};
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
///   `AssetServer`-present guard (`bevy-traps.md` #1): the theme spec (loaded by path),
///   and the weapon / armor / theme specs (each via `load_folder` of its OWN dedicated
///   compound extension — `weapon.ron` / `armor.ron` / `theme.ron` — so the `.ron`
///   folder dispatch is unambiguous, the game's loader scheme).
/// - `OnEnter(Load)`: kick off the loads.
/// - `Update` (while `Load` and any target resource is still absent): poll + resolve.
/// - `Update` (while `Load` and all four resources exist): transition to `Editing`.
pub(crate) fn register_load(app: &mut App) {
    if app.world().get_resource::<AssetServer>().is_some() {
        app.init_ron_asset::<GdtfThemeSpec>();
        app.init_ron_asset_with_extensions::<WeaponSpec>(vec!["weapon.ron"]);
        app.init_ron_asset_with_extensions::<ArmorSpec>(vec!["armor.ron"]);
        app.init_ron_asset_with_extensions::<ThemeSpec>(vec!["theme.ron"]);
    }

    app.add_systems(OnEnter(EditorState::Load), kick_off_editor_loads);
    app.add_systems(
        Update,
        (poll_and_resolve_editor, transition_to_editing)
            .chain()
            .run_if(in_state(EditorState::Load)),
    );
}
