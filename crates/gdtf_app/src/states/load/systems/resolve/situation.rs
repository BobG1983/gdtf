//! GTW-205 / GTW-261: resolves the authored [`Situation`] into the persistent,
//! gate-blocking [`LoadedSituation`] resource.

use bevy::{
    asset::LoadState,
    prelude::{AssetServer, Assets, Commands, warn},
};
use gdtf_assets::RonAsset;
use gdtf_battle_sim::situation::Situation;

use crate::states::load::resources::{LoadHandles, LoadedSituation};

/// GTW-205 / GTW-261: resolves the authored [`Situation`] RON into the persistent
/// [`LoadedSituation`] resource, mirroring the [`resolve_tuning`](super::resolve_tuning)
/// poll/resolve + warn/fallback shape.
///
/// As of GTW-261 the situation is a **gate-blocking** resource (the empty-battle-race
/// fix): the Load→Intro transition now requires a `LoadedSituation` (see the plugin
/// wiring), so a battle never starts before its real situation loads. This resolve
/// makes the situation exactly symmetric with the tuning and weapons branches — it
/// gates entry AND falls back to an empty default on failure, so a slow/failed
/// situation still always lets `Load` exit (the no-strand guarantee GTW-205 wanted,
/// preserved via the failure fallback rather than via non-blocking resolution).
///
/// Called only while no [`LoadedSituation`] resource exists yet (the caller's
/// own-absence guard), independently of the theme / tuning / weapons branches:
///
/// - If the situation RON reached [`LoadState::Failed`], `warn!`s naming
///   `situations/skirmish.ron` and inserts an empty [`Situation::default`] — the
///   ADR-0003 sanctioned error-path safety-net — so `Load` always exits with a
///   situation present and never hangs on a bad situation file. CRITICAL: the empty
///   default is inserted ONLY on a genuine `Failed`, NEVER while the situation is
///   still loading — inserting it early would clear the gate before the real
///   battlefield resolves, re-introducing the empty-battle bug.
/// - Else once the situation RON is [`LoadState::Loaded`], reads the deserialized
///   [`Situation`] out of `Assets<RonAsset<Situation>>` (the same transient-one-frame
///   `Assets::get` retry the theme path uses) and inserts the inner payload as the
///   persistent [`LoadedSituation`]. Like
///   [`GdtfTheme`](gdtf_ui::theme::GdtfTheme) it survives `OnExit(Load)` (it is
///   **not** removed in `cleanup`), because the Generation slice (E10.5) reads it.
/// - Else (still loading) it does nothing and is polled again next frame.
pub(super) fn resolve_situation(
    commands: &mut Commands,
    asset_server: &AssetServer,
    situation_assets: &Assets<RonAsset<Situation>>,
    handles: &LoadHandles,
) {
    let situation_state = asset_server.load_state(&*handles.situation);

    // Failure path: a bad/missing situation must not hang the app. Warn naming the
    // path and fall back to the empty default situation so Load always exits with one
    // present (a Failed → empty default; the battle then has zero gangers rather than
    // stranding the machine). Only on a genuine Failed — never while still loading.
    if situation_state.is_failed() {
        warn!(
            "GDTF Load: asset `situations/skirmish.ron` failed to load; falling back to the empty \
             default situation (the battle will have no gangers)",
        );
        commands.insert_resource(LoadedSituation::new(Situation::default()));
        return;
    }

    // Success path: once the situation RON is loaded, read the deserialized payload
    // out of its collection (transient-one-frame retry like the theme) and insert it
    // as the persistent LoadedSituation.
    if matches!(situation_state, LoadState::Loaded) {
        let Some(situation) = situation_assets.get(&*handles.situation) else {
            // Loaded-but-not-yet-in-collection — retry next frame (the system stays
            // alive while LoadedSituation is still absent).
            return;
        };
        // Persist the resolved battlefield for the Generation consumer (E10.5);
        // like GdtfTheme it survives OnExit(Load) (not removed in cleanup).
        commands.insert_resource(LoadedSituation::new((**situation).clone()));
    }
}
