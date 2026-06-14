//! `Update` (in `AppState::Load`): poll the loads, then resolve or fall back.

use bevy::{asset::LoadState, prelude::*};
use gdtf_assets::RonAsset;
use gdtf_ui::theme::{GdtfTheme, GdtfThemeSpec, default_theme};

use crate::scenes::load::resources::{FailedAssetPath, LoadFailed, LoadHandles};

/// Polls the in-flight loads and, once resolvable, inserts the [`GdtfTheme`].
///
/// Each frame, while [`LoadHandles`] exists and no [`GdtfTheme`] has been
/// inserted yet:
///
/// - If the theme RON **or** the font reached [`LoadState::Failed`], records the
///   failed path in a typed [`LoadFailed`] resource, `warn!`s naming it, and
///   inserts the const-fallback [`default_theme`] — the ADR-0003 sanctioned
///   error-path safety-net — so the app never hangs and never leaves `Load`
///   themeless.
/// - Else if the theme RON is [`LoadState::Loaded`], reads the deserialized
///   [`GdtfThemeSpec`] out of `Assets<RonAsset<GdtfThemeSpec>>` and resolves it
///   with the loaded font handle into a [`GdtfTheme`], then inserts it.
/// - Else (still loading) it does nothing and runs again next frame.
///
/// Guarded entirely by `run_if(resource_exists::<LoadHandles>)` plus the
/// `not(resource_exists::<GdtfTheme>)` gate in the plugin wiring, and takes
/// `Res<AssetServer>`/`Res<Assets<_>>`/`Res<LoadHandles>` — all of which are
/// present whenever those run-conditions hold, so it never panics on a missing
/// resource (bevy-traps rule 1). The early-`return`s on the run-condition
/// resources are belt-and-braces against a one-frame race.
pub(in crate::scenes::load) fn poll_and_resolve(
    mut commands: Commands,
    asset_server: Option<Res<AssetServer>>,
    theme_assets: Option<Res<Assets<RonAsset<GdtfThemeSpec>>>>,
    handles: Option<Res<LoadHandles>>,
) {
    let (Some(asset_server), Some(theme_assets), Some(handles)) =
        (asset_server, theme_assets, handles)
    else {
        return;
    };

    let theme_state = asset_server.load_state(&*handles.theme);
    let font_state = asset_server.load_state(&*handles.font);

    // Failure path: a failed required asset must not hang the app. Record the
    // failed path, warn, and fall back to the const default theme.
    if theme_state.is_failed() {
        fall_back(
            &mut commands,
            FailedAssetPath(String::from("theme/grimdark.ron")),
        );
        return;
    }
    if font_state.is_failed() {
        fall_back(
            &mut commands,
            FailedAssetPath(String::from("fonts/Alegreya-Variable.ttf")),
        );
        return;
    }

    // Success path: the theme RON is loaded — resolve it with the font handle.
    if matches!(theme_state, LoadState::Loaded) {
        let Some(spec) = theme_assets.get(&*handles.theme) else {
            // Loaded-but-not-yet-in-collection is a transient one-frame state;
            // try again next frame rather than failing.
            return;
        };
        let theme: GdtfTheme = (**spec).clone().resolve((*handles.font).clone());
        commands.insert_resource(theme);
    }
}

/// Records the failed asset path, warns naming it, and inserts the const-fallback
/// [`GdtfTheme`].
///
/// Shared by both failure branches so the warn-and-fallback is written once.
fn fall_back(commands: &mut Commands, path: FailedAssetPath) {
    warn!(
        "GDTF Load: asset `{}` failed to load; falling back to the const default theme",
        &*path,
    );
    commands.insert_resource(LoadFailed(path));
    commands.insert_resource(default_theme());
}
