//! `Update` (in `AppState::Load`): poll the loads, then resolve or fall back.

use bevy::{
    asset::{LoadState, RecursiveDependencyLoadState},
    prelude::*,
};
use gdtf_assets::RonAsset;
use gdtf_battle_sim::situation::Situation;
use gdtf_ui::theme::{ActiveThemeHandle, GdtfTheme, GdtfThemeSpec, default_theme};

use crate::scenes::load::resources::{FailedAssetPath, LoadFailed, LoadHandles, LoadedSituation};

/// Polls the in-flight loads and, once resolvable, inserts the [`GdtfTheme`].
///
/// Each frame, while [`LoadHandles`] exists and no [`GdtfTheme`] has been
/// inserted yet:
///
/// - If the theme RON reached [`LoadState::Failed`] **or** the fonts folder
///   reached [`RecursiveDependencyLoadState::Failed`], records the failed path in
///   a typed [`LoadFailed`] resource, `warn!`s naming it, and inserts the
///   const-fallback [`default_theme`] — the ADR-0003 sanctioned error-path
///   safety-net — so the app never hangs and never leaves `Load` themeless.
/// - Else once the theme RON is [`LoadState::Loaded`] **and** the fonts folder's
///   [`RecursiveDependencyLoadState`] is `Loaded` (recursive, so every font in
///   the folder is loaded — GTW-149), reads the deserialized [`GdtfThemeSpec`]
///   out of `Assets<RonAsset<GdtfThemeSpec>>` and resolves it with the font
///   resolver `|key| asset_server.load::<Font>(key)` into a [`GdtfTheme`], then
///   inserts it. `load` is idempotent — each font key returns its
///   already-preloaded handle.
/// - Else (still loading) it does nothing and runs again next frame.
///
/// On **both** the success and the failure paths it also inserts the persistent
/// [`ActiveThemeHandle`] (the theme RON handle from [`LoadHandles`]) alongside the
/// [`GdtfTheme`] — the handle is valid even when the load failed, so GTW-138's
/// later file-watcher reload can recover, and the GTW-137 live-retheme system
/// filters incoming asset events against it. Holding it keeps a strong reference
/// so the asset stays loaded for that watcher. Like [`GdtfTheme`], it persists
/// past `OnExit(Load)` (it is **not** removed in `cleanup`).
///
/// GTW-205 (E10.3): on the success path it ALSO resolves the authored
/// [`Situation`] into a persistent [`LoadedSituation`] — the source the Generation
/// slice (E10.5) reads. Once the situation RON reaches [`LoadState::Loaded`] it is
/// read out of `Assets<RonAsset<Situation>>` (the same transient-one-frame
/// `Assets::get` retry the theme path uses) and inserted as a [`LoadedSituation`]
/// that, like [`GdtfTheme`], persists past `OnExit(Load)`. This is deliberately
/// **non-blocking** to the theme: the situation resolution NEVER gates or alters
/// the existing theme-only Load→Intro transition. A situation that has not (yet)
/// reached `Loaded` simply leaves `LoadedSituation` un-inserted this pass and the
/// theme still resolves — so a slow/failed situation can never strand the machine
/// in `Load`.
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
    situation_assets: Option<Res<Assets<RonAsset<Situation>>>>,
    handles: Option<Res<LoadHandles>>,
) {
    let (Some(asset_server), Some(theme_assets), Some(situation_assets), Some(handles)) =
        (asset_server, theme_assets, situation_assets, handles)
    else {
        return;
    };

    let theme_state = asset_server.load_state(&*handles.theme);
    // Recursive (not direct) — gate on every font IN the folder being loaded.
    let fonts_state = asset_server.recursive_dependency_load_state(&*handles.fonts);

    // Failure path: a failed required asset must not hang the app. Record the
    // failed path, warn, and fall back to the const default theme. The active
    // theme handle is inserted even here — it is valid despite the failed load,
    // so a later file-watcher reload (GTW-138) can recover from it.
    if theme_state.is_failed() {
        fall_back(
            &mut commands,
            FailedAssetPath(String::from("theme/grimdark.ron")),
            &handles,
        );
        return;
    }
    if matches!(fonts_state, RecursiveDependencyLoadState::Failed(_)) {
        fall_back(
            &mut commands,
            FailedAssetPath(String::from("fonts")),
            &handles,
        );
        return;
    }

    // Success path: the theme RON is loaded AND every font in the folder is
    // loaded — resolve the spec, loading each font key idempotently.
    if matches!(theme_state, LoadState::Loaded)
        && matches!(fonts_state, RecursiveDependencyLoadState::Loaded)
    {
        // GTW-205 (E10.3): resolve the authored situation into the persistent
        // LoadedSituation FIRST, on the same success pass. Once it has reached
        // Loaded, read it out of its collection with the same transient-one-frame
        // retry the theme uses (loaded-but-not-yet-in-collection → return, retry
        // next frame). This is NON-BLOCKING to the theme: a situation that has not
        // yet reached Loaded just leaves LoadedSituation un-inserted this pass — so
        // it can never gate or alter the theme-only Load→Intro transition.
        if matches!(
            asset_server.load_state(&*handles.situation),
            LoadState::Loaded
        ) {
            let Some(situation) = situation_assets.get(&*handles.situation) else {
                // Loaded-but-not-yet-in-collection — retry next frame (the theme is
                // not inserted yet, so the run-condition keeps this system alive).
                return;
            };
            // Persist the resolved battlefield for the Generation consumer (E10.5);
            // like GdtfTheme it survives OnExit(Load) (not removed in cleanup).
            commands.insert_resource(LoadedSituation((**situation).clone()));
        }

        let Some(spec) = theme_assets.get(&*handles.theme) else {
            // Loaded-but-not-yet-in-collection is a transient one-frame state;
            // try again next frame rather than failing.
            return;
        };
        let theme: GdtfTheme = (**spec)
            .clone()
            .resolve(|key| asset_server.load::<Font>(key.to_owned()));
        commands.insert_resource(theme);
        // The persistent handle the GTW-137 retheme system filters against and
        // GTW-138's watcher keeps loaded; survives OnExit(Load) like GdtfTheme.
        commands.insert_resource(ActiveThemeHandle((*handles.theme).clone()));
    }
}

/// Records the failed asset path, warns naming it, and inserts the const-fallback
/// [`GdtfTheme`] plus the persistent [`ActiveThemeHandle`].
///
/// Shared by both failure branches so the warn-and-fallback is written once. The
/// [`ActiveThemeHandle`] is inserted on this path too: the handle is valid even
/// though the load failed, so GTW-138's file-watcher reload can recover from it.
fn fall_back(commands: &mut Commands, path: FailedAssetPath, handles: &LoadHandles) {
    warn!(
        "GDTF Load: asset `{}` failed to load; falling back to the const default theme",
        &*path,
    );
    commands.insert_resource(LoadFailed(path));
    commands.insert_resource(default_theme());
    commands.insert_resource(ActiveThemeHandle((*handles.theme).clone()));
}
