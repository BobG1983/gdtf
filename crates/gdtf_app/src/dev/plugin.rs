//! [`DevAffordancesPlugin`] — the ONE aggregate owner of the dev-QA affordances,
//! and the only piece of `crate::dev` that [`GdtfApp`](crate::GdtfApp) names.

use bevy::prelude::*;

/// Wires the DEV-ONLY affordances that must never reach a release artifact.
///
/// Gated on `cfg!(debug_assertions)` so a release build does not even compile the
/// auto-enter-battle affordance in ([`super::auto_battle`]). The affordance is
/// *additionally* inert by default at runtime — it activates only when
/// `GDTF_AUTOBATTLE` is set truthy
/// ([`auto_battle_enabled`](super::auto_battle::auto_battle_enabled)) — so a
/// normal `cargo run` (debug) still reaches the menu and stops. The `cfg` keeps the
/// release binary clean; the env gate keeps the dev launch inert until opted into.
///
/// The GTW-297 screenshot / visual-QA affordance (`super::capture`, which also
/// registers the `super::drive` battle-script triggers) is wired in beside it,
/// DOUBLE-gated on `cfg!(all(debug_assertions, feature = "dev_capture"))` — dev
/// feature AND debug build — so it never reaches a release artifact (release builds
/// enable neither). Since GTW-590 the binary's `dynamic_linking` dev feature folds
/// `dev_capture` in, so every dynamic-linked dev/gate build compiles it. It too is
/// inert at runtime until `GDTF_CAPTURE_PATH` is set. Paired with the auto-battle
/// affordance, it captures a live battlescape frame to a PNG unattended (see
/// `super::capture` for the invocation).
///
/// GTW-510: the interactive F10 screenshot keybind (captures the primary window to a
/// timestamped `target/screenshots/game-<secs>.png` without exiting) rides the same
/// double gate — the reusable `gdtf_screenshot` crate is pulled in only by the
/// `dev_capture` feature, so a release binary never links it.
///
/// A release build's `build` registers NOTHING — the plugin is indistinguishable
/// from not being added at all.
pub(crate) struct DevAffordancesPlugin;

impl Plugin for DevAffordancesPlugin {
    fn build(&self, app: &mut App) {
        #[cfg(debug_assertions)]
        app.add_plugins(super::auto_battle::AutoBattlePlugin::from_env());
        #[cfg(all(debug_assertions, feature = "dev_capture"))]
        app.add_plugins(super::capture::DevCapturePlugin::from_env());
        #[cfg(all(debug_assertions, feature = "dev_capture"))]
        app.add_plugins(gdtf_screenshot::KeyboardCapturePlugin::new("game"));
        #[cfg(not(debug_assertions))]
        let _ = app;
    }
}
