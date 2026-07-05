//! The DEV-ONLY screenshot / in-engine visual-QA affordance (GTW-297, extended GTW-306).
//!
//! This is **not shipping behavior**. It exists solely so a coding agent (and the
//! orchestrator's post-gate QA) can drive the app into a live, rendered battle and
//! capture the rendered frame(s) to PNG(s) on disk — which the agent then `Read`s to
//! visually verify the HUD / FX that the per-slice headless tests structurally cannot
//! observe. It pairs with the GTW-223 [`AutoBattlePlugin`](crate::app::auto_battle)
//! which drives the app unattended into [`BattleScapeState::BattleRunning`].
//!
//! ## Two gates, both must hold to activate
//!
//! 1. **Dev cfg.** The affordance is wired into [`GdtfApp`](crate::GdtfApp) only
//!    under `cfg!(all(debug_assertions, feature = "dev_capture"))` — a debug / dev
//!    build with the opt-in `dev_capture` feature. A release artifact never sees it
//!    even if the feature is on, and the default suite never compiles it.
//! 2. **Opt-in env var(s).** Even when compiled in it is **inert by default**: each
//!    sub-affordance activates only when its env var is set. With them all unset the
//!    plugin registers nothing.
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
//!   Independent of capture (it activates on its own env var).
//!
//! ## How it captures
//!
//! When a capture path is present, [`DevCapturePlugin`] registers ONE `Update` system,
//! [`capture_when_ready`], gated `run_if(in_state(BattleScapeState::BattleRunning))`.
//! It carries a [`Local<u32>`] frame counter that increments only while the battle is
//! running (so it counts frames since the battle rendered, not total app frames; a
//! `Res<FrameCount>` would hang on macOS, Bevy issue #24035). On each frame matching a
//! target in [`CaptureFrames`] it spawns a [`Screenshot::primary_window`](bevy::render::view::window::screenshot::Screenshot::primary_window) entity with an
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
//! `Read`s the PNGs). The headless tests cover the [`DevCapturePlugin::from_env`] config
//! logic (path gate, [`CaptureFrame`](super::capture_config::CaptureFrame) / [`CaptureFrames`] / [`FireAtFrame`] parse) AND
//! the fire-trigger's frame match + real-path message emission (which IS headless).
//!
//! ## Visibility
//!
//! Unlike the GTW-223 `auto_battle` affordance — whose plugin the EXTERNAL
//! `test_support` / `gdtf_test_utils` harness names (so it widens to `pub` via
//! `support_item!`) — NOTHING outside this crate references the capture items: the
//! binary wires [`DevCapturePlugin`] in-crate via `gdtf_app.rs`, and the tests are the
//! in-crate `#[cfg(test)]` sibling. So every item here is `pub(crate)` (the widest
//! visibility anything reaches), keeping the binary `unreachable_pub`-clean WITHOUT the
//! `support_item!` flip.

use std::path::PathBuf;

use bevy::prelude::*;
use gdtf_battle_sim::apply_falls;

use super::{
    capture_config::{CaptureFrames, capture_path},
    screenshot::capture_when_ready,
    trigger_config::{FallAtFrame, FireAtFrame, FireModeOverride},
    triggers::{trigger_fall_at_frame, trigger_fire_at_frame},
};
use crate::states::BattleScapeState;

/// The resolved capture configuration: where to write and which frames to capture.
///
/// Held by [`DevCapturePlugin`] when the capture sub-affordance is enabled, and inserted
/// as a [`Resource`] so [`capture_when_ready`] can read both fields. Framework-plumbing
/// config (a path + a [`CaptureFrames`] newtype), not a domain value, so the
/// no-bare-types rule applies only to its `frames` field (which is the newtype).
/// `pub(crate)`: a purely internal resource, never re-exported.
#[derive(Resource, Debug, Clone)]
pub(crate) struct CaptureConfig {
    /// Absolute base path of the output PNG(s), handed to
    /// [`save_to_disk`](bevy::render::view::window::screenshot::save_to_disk). On the
    /// multi-frame path each frame is suffixed via [`frame_path`](super::capture_config::frame_path).
    pub(super) path:   PathBuf,
    /// Which `BattleRunning` frames to capture (one PNG per frame).
    pub(super) frames: CaptureFrames,
}

/// The resolved fire-trigger configuration: the frame to fire on.
///
/// Held by [`DevCapturePlugin`] when the fire sub-affordance is enabled, inserted as a
/// [`Resource`] so [`trigger_fire_at_frame`] can read it. `pub(crate)`: internal only.
#[derive(Resource, Debug, Clone, Copy)]
pub(crate) struct FireConfig {
    /// The `BattleRunning` frame at which the selected player ganger fires.
    pub(super) frame: FireAtFrame,
    /// An optional fire-MODE override (`GDTF_FIRE_MODE`): when set, the trigger fires in
    /// this authored [`ModeKind`](gdtf_battle_sim::ModeKind) (read off the shooter's [`FireMode`](gdtf_battle_sim::FireMode)) rather than the
    /// resident [`SelectedFireMode`](gdtf_battle_input::SelectedFireMode) — the FX-capture path uses `Full` for a staggered
    /// multi-round volley.
    pub(super) mode:  Option<FireModeOverride>,
}

impl FireConfig {
    /// Build a fire-trigger config from the frame to fire on and an optional mode override.
    /// Test-only inherent surface (the production path constructs it via struct literal in
    /// `from_env`; the headless test seeds the REAL resource the [`trigger_fire_at_frame`]
    /// system reads through this). `#[cfg(test)]` so the binary stays `dead_code`-clean.
    #[cfg(test)]
    #[must_use]
    pub(crate) const fn new(frame: FireAtFrame, mode: Option<FireModeOverride>) -> Self {
        Self { frame, mode }
    }
}

/// The resolved fall-trigger configuration: the frame to force a fall on.
///
/// Held by [`DevCapturePlugin`] when the fall sub-affordance is enabled, inserted as a
/// [`Resource`] so [`trigger_fall_at_frame`] can read it. `pub(crate)`: internal only.
/// Mirrors [`FireConfig`].
#[derive(Resource, Debug, Clone, Copy)]
pub(crate) struct FallConfig {
    /// The `BattleRunning` frame at which the chosen player ganger is forced to fall.
    pub(super) frame: FallAtFrame,
}

impl FallConfig {
    /// Build a fall-trigger config from the frame to force a fall on. Test-only inherent
    /// surface (the production path constructs it via struct literal in `from_env`; the
    /// headless test seeds the REAL resource the [`trigger_fall_at_frame`] system reads
    /// through this). `#[cfg(test)]` so the binary stays `dead_code`-clean.
    #[cfg(test)]
    #[must_use]
    pub(crate) const fn new(frame: FallAtFrame) -> Self {
        Self { frame }
    }
}

/// The DEV-ONLY screenshot / visual-QA + fire-trigger affordance plugin (GTW-297 /
/// GTW-306).
///
/// Wired into [`GdtfApp`](crate::GdtfApp) only under `cfg!(all(debug_assertions, feature
/// = "dev_capture"))`. On `build` it consults its config; for each present sub-config it
/// registers the matching `Update` system, gated on [`BattleScapeState::BattleRunning`].
/// When NO sub-config is present it registers nothing — the affordance is fully inert,
/// exactly like a build without the plugin. `pub(crate)`: named only by the in-crate
/// wiring + config tests.
pub(crate) struct DevCapturePlugin {
    /// The capture configuration, captured once at construction. `None` = no capture.
    capture: Option<CaptureConfig>,
    /// The fire-trigger configuration. `None` = no dev fire trigger.
    fire:    Option<FireConfig>,
    /// The fall-trigger configuration (GTW-529). `None` = no dev fall trigger.
    fall:    Option<FallConfig>,
}

impl DevCapturePlugin {
    /// Construct the affordance, reading every env-var gate: the capture path
    /// ([`capture_path`]) + frame schedule ([`CaptureFrames::from_env`]) and the
    /// fire-trigger frame ([`FireAtFrame::from_env`]).
    ///
    /// This is the constructor [`GdtfApp`](crate::GdtfApp) uses under
    /// `cfg!(all(debug_assertions, feature = "dev_capture"))`: each env var decides
    /// whether its sub-affordance activates, so the plugin is inert on a normal launch
    /// (the variables unset).
    #[must_use]
    pub(crate) fn from_env() -> Self {
        Self {
            capture: capture_path().map(|path| CaptureConfig {
                path,
                frames: CaptureFrames::from_env(),
            }),
            fire:    FireAtFrame::from_env().map(|frame| FireConfig {
                frame,
                mode: FireModeOverride::from_env(),
            }),
            fall:    FallAtFrame::from_env().map(|frame| FallConfig { frame }),
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

    /// The configured [`CaptureFrames`] schedule, if the capture sub-affordance is
    /// enabled. Test-only inherent surface. `#[cfg(test)]`.
    #[cfg(test)]
    #[must_use]
    pub(crate) fn capture_frames(&self) -> Option<CaptureFrames> {
        self.capture.as_ref().map(|config| config.frames.clone())
    }

    /// The configured [`FireAtFrame`], if the fire sub-affordance is enabled. Test-only
    /// inherent surface. `#[cfg(test)]`.
    #[cfg(test)]
    #[must_use]
    pub(crate) fn fire_frame(&self) -> Option<FireAtFrame> {
        self.fire.map(|config| config.frame)
    }

    /// The configured [`FallAtFrame`], if the fall sub-affordance is enabled. Test-only
    /// inherent surface (GTW-529). `#[cfg(test)]`.
    #[cfg(test)]
    #[must_use]
    pub(crate) fn fall_frame(&self) -> Option<FallAtFrame> {
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
        info!("dev-capture: ON (dev)");
        if let Some(config) = self.capture.clone() {
            app.insert_resource(config).add_systems(
                Update,
                capture_when_ready.run_if(in_state(BattleScapeState::BattleRunning)),
            );
        }
        if let Some(config) = self.fire {
            app.insert_resource(config).add_systems(
                Update,
                trigger_fire_at_frame.run_if(in_state(BattleScapeState::BattleRunning)),
            );
        }
        if let Some(config) = self.fall {
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
