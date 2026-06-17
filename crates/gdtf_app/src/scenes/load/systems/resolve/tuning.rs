//! GTW-206 (E10.4): resolves the shipped [`CombatTuning`] into the persistent runtime
//! [`CombatTuning`] resource.

use bevy::{
    asset::LoadState,
    prelude::{AssetServer, Assets, Commands, warn},
};
use gdtf_assets::RonAsset;
use gdtf_battle_sim::tuning::CombatTuning;

use crate::scenes::load::resources::LoadHandles;

/// GTW-206 (E10.4): resolves the shipped [`CombatTuning`] RON into the persistent
/// runtime [`CombatTuning`] resource, mirroring the theme path's poll/resolve +
/// warn/fallback shape but for a payload that needs NO `resolve()` step
/// (`CombatTuning` is BOTH the `Deserialize` payload AND the `Resource`).
///
/// Called only while no [`CombatTuning`] resource exists yet (the caller's
/// own-absence guard), independently of the theme branch:
///
/// - If the tuning RON reached [`LoadState::Failed`], `warn!`s naming
///   `combat/tuning.ron` and inserts [`CombatTuning::default`] — the ADR-0003
///   sanctioned error-path safety-net — so `Load` always exits with a tuning
///   present and never hangs on a bad tuning file.
/// - Else once the tuning RON is [`LoadState::Loaded`], reads the deserialized
///   [`CombatTuning`] out of `Assets<RonAsset<CombatTuning>>` (the same
///   transient-one-frame `Assets::get` retry the theme path uses) and inserts the
///   inner payload directly as the persistent resource. Like
///   [`GdtfTheme`](gdtf_ui::theme::GdtfTheme) it survives `OnExit(Load)` (it is
///   **not** removed in `cleanup`), because `BattleScape` reads it.
/// - Else (still loading) it does nothing and is polled again next frame.
pub(super) fn resolve_tuning(
    commands: &mut Commands,
    asset_server: &AssetServer,
    tuning_assets: &Assets<RonAsset<CombatTuning>>,
    handles: &LoadHandles,
) {
    let tuning_state = asset_server.load_state(&*handles.tuning);

    // Failure path: a bad tuning must not hang the app. Warn naming the path and
    // fall back to the const-default tuning so Load always exits with one present.
    if tuning_state.is_failed() {
        warn!(
            "GDTF Load: asset `combat/tuning.ron` failed to load; falling back to the const \
             default combat tuning",
        );
        commands.insert_resource(CombatTuning::default());
        return;
    }

    // Success path: once the tuning RON is loaded, read the deserialized payload
    // out of its collection (transient-one-frame retry like the theme) and insert
    // it directly — CombatTuning is both the payload and the runtime resource.
    if matches!(tuning_state, LoadState::Loaded) {
        let Some(tuning) = tuning_assets.get(&*handles.tuning) else {
            // Loaded-but-not-yet-in-collection — retry next frame (the system stays
            // alive while CombatTuning is still absent).
            return;
        };
        commands.insert_resource((**tuning).clone());
    }
}
