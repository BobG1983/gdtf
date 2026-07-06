//! The DEV-ONLY screenshot / in-engine visual-QA affordance (GTW-297, extended GTW-306).
//!
//! This is **not shipping behavior**. It exists solely so a coding agent (and the
//! orchestrator's post-gate QA) can drive the app into a live, rendered battle and
//! capture the rendered frame(s) to PNG(s) on disk — which the agent then `Read`s to
//! visually verify the HUD / FX that the per-slice headless tests structurally cannot
//! observe. It pairs with the GTW-223 [`AutoBattlePlugin`](crate::dev::auto_battle)
//! which drives the app unattended into [`BattleScapeState::BattleRunning`].
//!
//! ## Two gates, both must hold to activate
//!
//! 1. **Dev cfg.** The affordance is wired into [`GdtfApp`](crate::GdtfApp) only
//!    under `cfg!(all(debug_assertions, feature = "dev_capture"))` — a debug / dev
//!    build with the `dev_capture` feature. Since GTW-590 the BINARY's
//!    `dynamic_linking` dev feature folds `dev_capture` in, so every dynamic-linked
//!    dev invocation (and the dclippy/dtest gate suite) compiles it; a release
//!    artifact (never dynamic-linked) still never sees it.
//! 2. **Opt-in env var(s).** Even when compiled in it is **inert by default**: each
//!    sub-affordance activates only when its env var is set. With them all unset the
//!    plugin registers nothing.
//!
//! ## Loudness contract (GTW-590 C3)
//!
//! Every activation, trigger, write, and failure is one `grep -i capture` away:
//!
//! - a set-but-ineffective env var `warn!`s at construction (the
//!   [`CaptureConfigWarning`](super::resolve::CaptureConfigWarning) diagnostics from
//!   the sibling `resolve` module);
//! - each active sub-affordance `info!`s its resolved config at registration
//!   (`dev-capture: capture ON -> ...` / `... fire trigger ON ...` / `... fall
//!   trigger ON ...`), and a capture output directory that cannot be created is an
//!   immediate `error!` naming path + cause;
//! - each scheduled frame `info!`s `dev-capture: capturing BattleRunning frame N ->
//!   path` when it fires, and the triggers `warn!` when their one shot is skipped
//!   (no shooter / faction / enemy) instead of no-oping silently;
//! - the WRITE result is bevy's own
//!   [`save_to_disk`](bevy::render::view::window::screenshot::save_to_disk) observer
//!   log — `Screenshot saved to <path>` on success, `Cannot save screenshot ...` at
//!   `error!` on any failure (`bevy_render` 0.19.0 `view/window/screenshot.rs`) — one
//!   line per written frame.
//!
//! ## The three sub-affordances (GTW-306)
//!
//! - **Single-frame capture** — `GDTF_CAPTURE_PATH=/abs/out.png` plus an optional
//!   `GDTF_CAPTURE_FRAME=<n>` (default [`CaptureFrame::DEFAULT`](super::capture_config::CaptureFrame::DEFAULT)): captures ONE frame
//!   to the exact path. Unchanged from GTW-297.
//! - **Multi-frame capture** — `GDTF_CAPTURE_PATH=/abs/out.png` plus
//!   `GDTF_CAPTURE_FRAMES="12,14,16,18"`: captures EACH listed `BattleRunning` frame to
//!   its own PNG (`out.f12.png`, `out.f14.png`, …), then exits after the last. The
//!   frame list wins over `GDTF_CAPTURE_FRAME` when both are set. This is the FX QA path
//!   (GTW-306): a frame SEQUENCE of one shot in a single run.
//! - **Fire trigger** — `GDTF_FIRE_AT_FRAME=<n>`: at `BattleRunning` frame `n` the
//!   selected player ganger fires at the nearest enemy via the REAL fire path (it writes
//!   a [`FireRequested`](gdtf_battle_sim::acts::FireRequested) message — exactly what a
//!   left-click fire produces — so the sim's `dispatch_fire` resolves the shot, the FX
//!   slices emit their projectile / impact, and the capture sequence above records it).
//!   Independent of capture (its own env var); a DRIVE affordance owned by the sibling
//!   `crate::dev::drive` module (GTW-632), this plugin remaining its one registrar.
//!
//! ## How it captures
//!
//! When a capture path is present, [`DevCapturePlugin`] registers ONE `Update` system,
//! [`capture_when_ready`], gated `run_if(in_state(BattleScapeState::BattleRunning))`.
//! It carries a [`Local<u32>`] frame counter that increments only while the battle is
//! running (so it counts frames since the battle rendered, not total app frames; a
//! `Res<FrameCount>` would hang on macOS, Bevy issue #24035). On each frame matching a
//! target in [`CaptureFrames`](super::capture_config::CaptureFrames) it spawns a [`Screenshot::primary_window`](bevy::render::view::window::screenshot::Screenshot::primary_window) entity with an
//! observer that saves the frame to that frame's PNG path; on the LAST target frame the
//! observer hands off to the shutdown cascade by setting
//! [`RunningState::Quit`](crate::states::RunningState::Quit).
//!
//! ## How it exits (GTW-316)
//!
//! The capture completion rides the SAME shutdown cascade as the battle aftermath instead
//! of writing `AppExit` directly: the
//! [`ScreenshotCaptured`](bevy::render::view::window::screenshot::ScreenshotCaptured)
//! observer saves the final PNG synchronously, then on the last scheduled frame sets
//! [`RunningState::Quit`](crate::states::RunningState::Quit). That advances
//! [`AppState::Teardown`](crate::states::AppState::Teardown), whose scene despawns the
//! `PrimaryWindow` (windowed/macOS native exit, no hang) and writes `AppExit` (headless
//! fallback). Writing `AppExit` from an ordinary observer does NOT reliably terminate
//! winit on macOS (Bevy issue #23313, unfixed in 0.18.1) — that is what made the capture
//! run hang at its last frame. GTW-311 fixed the teardown-driven exit; GTW-316 routes
//! this sibling capture path through the same cascade.
//!
//! The actual screenshot capture needs a real render device, so it CANNOT be
//! headless-tested — it is verified by RUNNING the app (the orchestrator does so, then
//! `Read`s the PNGs). The headless tests cover the [`resolve`](super::resolve::resolve)
//! config path (parse gates + the GTW-590 loud diagnostics), the registration /
//! state-gating of [`capture_when_ready`] (the schedule pin), the fire-trigger's
//! frame match + real-path message emission (which IS headless), AND the loud-line
//! EMISSION itself ([`warn_config_diagnostics`], the blocked-output-dir `error!`, a
//! trigger skip `warn!`) under the shared log capture — the `test::loudness` pins.
//!
//! ## Visibility
//!
//! Unlike the GTW-223 `auto_battle` affordance — whose plugin the EXTERNAL
//! `test_support` / `gdtf_test_utils` harness names (so it widens to `pub` via
//! `support_item!`) — NOTHING outside this crate references the capture items: the
//! binary wires [`DevCapturePlugin`] in-crate via the dev aggregate plugin
//! (`crate::dev::plugin`), and the tests are the in-crate `#[cfg(test)]` sibling. So
//! every item here is `pub(crate)` (the widest
//! visibility anything reaches), keeping the binary `unreachable_pub`-clean WITHOUT the
//! `support_item!` flip.

use bevy::prelude::*;
use gdtf_battle_sim::falls::apply_falls;

use super::{
    capture_config::CaptureConfig,
    diagnostics::{ensure_output_dir, warn_config_diagnostics},
    resolve::{RawCaptureEnv, ResolvedCaptureEnv, resolve},
    screenshot::capture_when_ready,
};
use crate::{
    dev::drive::{
        trigger_config::{FallConfig, FireConfig},
        triggers::{trigger_fall_at_frame, trigger_fire_at_frame},
    },
    states::BattleScapeState,
};

/// The DEV-ONLY screenshot / visual-QA + fire-trigger affordance plugin (GTW-297 /
/// GTW-306).
///
/// Wired into [`GdtfApp`](crate::GdtfApp) only under `cfg!(all(debug_assertions, feature
/// = "dev_capture"))`. On `build` it consults its config; for each present sub-config it
/// registers the matching `Update` system, gated on [`BattleScapeState::BattleRunning`],
/// and `info!`s the resolved config (the GTW-590 loudness contract — see the module
/// doc). When NO sub-config is present it registers nothing — the affordance is fully
/// inert, exactly like a build without the plugin. `pub(crate)`: named only by the
/// in-crate wiring + config tests.
pub(crate) struct DevCapturePlugin {
    /// The capture configuration, captured once at construction. `None` = no capture.
    capture: Option<CaptureConfig>,
    /// The fire-trigger configuration. `None` = no dev fire trigger.
    fire:    Option<FireConfig>,
    /// The fall-trigger configuration (GTW-529). `None` = no dev fall trigger.
    fall:    Option<FallConfig>,
}

impl DevCapturePlugin {
    /// Construct the affordance from the env-var gates: snapshot the vars once
    /// ([`RawCaptureEnv::from_env`]), [`resolve`] the snapshot purely, and `warn!`
    /// every [`CaptureConfigWarning`](super::resolve::CaptureConfigWarning)
    /// diagnostic via [`warn_config_diagnostics`] (GTW-590 — a set-but-ineffective
    /// var is never silent).
    ///
    /// This is the constructor [`GdtfApp`](crate::GdtfApp) uses under
    /// `cfg!(all(debug_assertions, feature = "dev_capture"))`: each env var decides
    /// whether its sub-affordance activates, so the plugin is inert on a normal launch
    /// (the variables unset).
    #[must_use]
    pub(crate) fn from_env() -> Self {
        let resolved = resolve(&RawCaptureEnv::from_env());
        warn_config_diagnostics(&resolved);
        Self::from_resolved(resolved)
    }

    /// Build the plugin from an already-resolved env snapshot — the pure tail of
    /// [`DevCapturePlugin::from_env`], split out so the headless tests construct the
    /// REAL plugin from an injected [`RawCaptureEnv`] (no process-global env
    /// mutation). Drops the diagnostics: the caller owns logging them.
    #[must_use]
    pub(super) fn from_resolved(resolved: ResolvedCaptureEnv) -> Self {
        Self {
            capture: resolved.capture,
            fire:    resolved.fire,
            fall:    resolved.fall,
        }
    }

    /// Whether the CAPTURE sub-affordance will activate on `build` (a path is
    /// configured).
    ///
    /// Test-only inherent surface (the config tests assert the gate without touching a
    /// process-global env var). `#[cfg(test)]` so the binary never compiles it (keeping
    /// it `dead_code`-clean).
    #[cfg(test)]
    #[must_use]
    pub(crate) const fn capture_enabled(&self) -> bool {
        self.capture.is_some()
    }

    /// The configured [`CaptureFrames`](super::capture_config::CaptureFrames)
    /// schedule, if the capture sub-affordance is enabled. Test-only inherent surface.
    /// `#[cfg(test)]`.
    #[cfg(test)]
    #[must_use]
    pub(crate) fn capture_frames(&self) -> Option<super::capture_config::CaptureFrames> {
        self.capture.as_ref().map(|config| config.frames.clone())
    }

    /// The configured [`FireAtFrame`](crate::dev::drive::trigger_config::FireAtFrame),
    /// if the fire sub-affordance is enabled. Test-only inherent surface. `#[cfg(test)]`.
    #[cfg(test)]
    #[must_use]
    pub(crate) fn fire_frame(&self) -> Option<crate::dev::drive::trigger_config::FireAtFrame> {
        self.fire.map(|config| config.frame)
    }

    /// The configured [`FallAtFrame`](crate::dev::drive::trigger_config::FallAtFrame),
    /// if the fall sub-affordance is enabled. Test-only inherent surface (GTW-529).
    /// `#[cfg(test)]`.
    #[cfg(test)]
    #[must_use]
    pub(crate) fn fall_frame(&self) -> Option<crate::dev::drive::trigger_config::FallAtFrame> {
        self.fall.map(|config| config.frame)
    }
}

impl Default for DevCapturePlugin {
    /// The wiring default: read the env-var gates.
    fn default() -> Self {
        Self::from_env()
    }
}

impl Plugin for DevCapturePlugin {
    fn build(&self, app: &mut App) {
        if self.capture.is_none() && self.fire.is_none() && self.fall.is_none() {
            // Inert: register nothing. The app runs normally and never captures / fires / falls.
            return;
        }
        if let Some(config) = self.capture.clone() {
            // GTW-590 C3: materialize the output directory NOW, loudly — before the fix a
            // missing parent surfaced only as bevy's write-time IO error (and before that,
            // nothing at all).
            if let Err(cause) = ensure_output_dir(&config) {
                error!(
                    "dev-capture: cannot create the capture output directory for {}: {cause}",
                    config.path.display(),
                );
            }
            info!(
                "dev-capture: capture ON -> {} at BattleRunning frame(s) {}",
                config.path.display(),
                config.frames,
            );
            app.insert_resource(config).add_systems(
                Update,
                capture_when_ready.run_if(in_state(BattleScapeState::BattleRunning)),
            );
        }
        if let Some(config) = self.fire {
            info!(
                "dev-capture: fire trigger ON at BattleRunning frame {} (mode override {:?})",
                *config.frame,
                config.mode.map(|mode| *mode),
            );
            app.insert_resource(config).add_systems(
                Update,
                trigger_fire_at_frame.run_if(in_state(BattleScapeState::BattleRunning)),
            );
        }
        if let Some(config) = self.fall {
            info!(
                "dev-capture: fall trigger ON at BattleRunning frame {}",
                *config.frame,
            );
            // GTW-529 C4 / `bevy-traps.md` #3: order the trigger `.before(apply_falls)` so the
            // SAME-FRAME `SlabDestroyed` it writes is buffered AND its elevating `Position`
            // rewrite is visible when the GTW-523 `apply_falls` reads the faller query — the fall
            // resolves the very frame the slab is smashed, so the drop + the GTW-524 impact flash
            // land in the captured frame (C3). `apply_falls` is only registered inside the battle
            // window (its `BattleInProgress`-gated `SimSystems::Simulate` band), so the
            // `.before` is a soft ordering constraint that binds whenever both run.
            app.insert_resource(config).add_systems(
                Update,
                trigger_fall_at_frame
                    .before(apply_falls)
                    .run_if(in_state(BattleScapeState::BattleRunning)),
            );
        }
    }
}
