//! [`DevAffordancesPlugin`] — the ONE aggregate owner of the dev-QA affordances,
//! and the only piece of `crate::dev` that [`GdtfApp`](crate::GdtfApp) names.

use bevy::prelude::*;

/// Wires the DEV-ONLY affordances that must never reach a release artifact.
///
/// GTW-655: the procgen load-time stepper (`super::procgen_stepper`) is `dev_tools`-gated
/// only (the feature pulls in `bevy_egui`) — no `debug_assertions` double-gate, since
/// `dev_tools` is itself never enabled for a release/CI build. It stays inert at runtime
/// (a battle loads exactly as it does without `dev_tools`) until `GDTF_PROCGEN_STEPPER` is
/// set truthy (its own `from_env` gate).
///
/// GTW-864: this plugin is also the ONE place in `gdtf_app` that adds `EguiPlugin`. With a
/// single adder there is no duplicate add to defend against: the egui-using affordances
/// (currently only the procgen stepper) draw into the context created here and add no plugin
/// of their own. The add carries its companion context binding (`super::egui_context`), since
/// disabling `auto_create_primary_context` is what makes an explicit binding owed. It rides
/// the `dev_tools` gate (that feature is what pulls `bevy_egui` in at all), is skipped under
/// `test-support` (the headless harness has no primary window for egui to draw into), and
/// needs Bevy's render stack — `EguiPlugin` reads `Assets<Shader>`, which exists only once
/// [`RenderPlugin`](bevy::render::RenderPlugin) is in the app.
///
/// GTW-736 (extended GTW-749): the QA network control channel (`super::net_qa`, the
/// GTW-694 architecture's T3 onward) is DOUBLE-gated —
/// `cfg!(all(debug_assertions, feature = "net_qa"))` — because it opens a loopback TCP
/// listener. It too is inert at runtime until `GDTF_NET_QA` is set truthy (its own
/// `from_env` gate); the listen port is `GDTF_NET_QA_PORT` (port only — the interface is
/// always `Ipv4Addr::LOCALHOST`). This is the ONE capture / drive path now: GTW-749
/// retired the `GDTF_AUTOBATTLE` auto-enter-battle drive and the `GDTF_CAPTURE_*` /
/// `GDTF_FIRE_AT_FRAME` / `GDTF_FIRE_MODE` env-var capture/drive rig outright, so a
/// coding-agent QA harness now boots the game and drives it entirely over this channel
/// (`launch_game` → `StartBattle` → `send_input` / `screenshot_after` → `stop_game`).
///
/// GTW-510: the interactive F10 screenshot keybind (captures the primary window to a
/// timestamped `target/screenshots/game-<secs>.png` without exiting) rides the SAME
/// double gate as the QA channel — the reusable `gdtf_screenshot` crate is pulled in by
/// the `net_qa` feature (its T7 screenshot pump's own dependency), so a release binary
/// never links it.
///
/// A release build's `build` registers NOTHING — the plugin is indistinguishable
/// from not being added at all.
pub(crate) struct DevAffordancesPlugin;

impl Plugin for DevAffordancesPlugin {
    fn build(&self, app: &mut App) {
        // GTW-864: the ONE `EguiPlugin` add, made here before any egui-using affordance is
        // wired (see the struct doc). It also takes control of WHERE the primary egui context
        // lives, which is why the companion binding system is registered right beside it.
        #[cfg(all(feature = "dev_tools", not(feature = "test-support")))]
        {
            use bevy::render::RenderPlugin;
            use bevy_egui::{EguiGlobalSettings, EguiPlugin};

            // A RENDER-STACK PRECONDITION, not a duplicate-add guard: `EguiPlugin` reads
            // `Assets<Shader>`, which exists only once Bevy's `RenderPlugin` is in the app, so
            // adding egui to a `MinimalPlugins` app panics on the first frame. A real binary
            // always has the render stack, so shipped wiring is unaffected.
            if app.is_plugin_added::<RenderPlugin>() {
                app.add_plugins(EguiPlugin::default());
                app.insert_resource(EguiGlobalSettings {
                    auto_create_primary_context: false,
                    ..default()
                });
                app.add_systems(Update, super::egui_context::bind_primary_egui_context);
            }
        }
        // GTW-655: the procgen load-time stepper. `dev_tools`-gated only (see the module
        // doc) — its own `from_env` env-var gate keeps it inert at runtime by default.
        #[cfg(feature = "dev_tools")]
        app.add_plugins(super::procgen_stepper::ProcgenStepperPlugin::from_env());
        // GTW-736/GTW-749: the QA network control channel — the ONE capture / drive
        // path. DOUBLE-gated `all(debug_assertions, feature = "net_qa")` (it opens a
        // listener); its own `from_env` `GDTF_NET_QA` gate keeps it inert at runtime by
        // default.
        #[cfg(all(debug_assertions, feature = "net_qa"))]
        app.add_plugins(super::net_qa::NetQaPlugin::from_env());
        // GTW-510: the F10 debug keybind, riding the SAME double gate as the QA channel
        // (see the struct doc).
        #[cfg(all(debug_assertions, feature = "net_qa"))]
        app.add_plugins(gdtf_screenshot::KeyboardCapturePlugin::new("game"));
        #[cfg(not(any(feature = "dev_tools", all(debug_assertions, feature = "net_qa"))))]
        let _ = app;
    }
}
