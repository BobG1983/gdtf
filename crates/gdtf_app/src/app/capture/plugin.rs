//! The DEV-ONLY screenshot / in-engine visual-QA affordance (GTW-297).
//!
//! This is **not shipping behavior**. It exists solely so a coding agent (and the
//! orchestrator's post-gate QA) can drive the app into a live, rendered battle and
//! capture the rendered frame to a PNG on disk — which the agent then `Read`s to
//! visually verify the HUD that the per-slice headless tests structurally cannot
//! observe. It pairs with the GTW-223 [`AutoBattlePlugin`](crate::app::auto_battle)
//! which drives the app unattended into [`BattleScapeState::BattleRunning`].
//!
//! ## Two gates, both must hold to activate
//!
//! 1. **Dev cfg.** The affordance is wired into [`GdtfApp`](crate::GdtfApp) only
//!    under `cfg!(all(debug_assertions, feature = "dev_capture"))` — a debug / dev
//!    build with the opt-in `dev_capture` feature. A release artifact never sees it
//!    even if the feature is on, and the default suite never compiles it.
//! 2. **Opt-in env var.** Even when compiled in it is **inert by default**: it
//!    activates only when `GDTF_CAPTURE_PATH` is set to the absolute path of the
//!    output PNG ([`capture_path`]). With it unset the plugin registers nothing.
//!
//! ## How it captures
//!
//! When active, [`DevCapturePlugin`] registers ONE `Update` system,
//! [`capture_when_ready`], gated `run_if(in_state(BattleScapeState::BattleRunning))`.
//! It carries a [`Local<u32>`] frame counter that increments only while the battle is
//! running (so it counts frames since the battle rendered, not total app frames; a
//! `Res<FrameCount>` would hang on macOS, Bevy issue #24035). Once the counter reaches
//! the configured [`CaptureFrame`] (default [`CaptureFrame::DEFAULT`]) — giving Bevy's
//! UI layout time to flush — it spawns a [`Screenshot::primary_window`] entity with an
//! observer that, when the frame is captured, saves it to disk and writes
//! `AppExit::Success`.
//!
//! `AppExit::Success` is written ONLY from inside the
//! [`ScreenshotCaptured`](bevy::render::view::window::screenshot::ScreenshotCaptured)
//! observer (the `ci_testing` pattern) to mitigate the macOS `AppExit` hang (Bevy issue
//! #23313, not fixed in 0.18.1). If a future Bevy still hangs there, the documented
//! fallback is to despawn the `PrimaryWindow` entity to take winit's native exit path.
//!
//! The actual screenshot capture needs a real render device, so it CANNOT be
//! headless-tested — it is verified by RUNNING the app (the orchestrator does so, then
//! `Read`s the PNG). The headless tests cover only the [`DevCapturePlugin::from_env`]
//! config logic ([`capture_path`] gating + [`CaptureFrame`] default / parse).
//!
//! ## Visibility
//!
//! Unlike the GTW-223 `auto_battle` affordance — whose plugin the EXTERNAL
//! `test_support` / `gdtf_test_utils` harness names (so it widens to `pub` via
//! `support_item!`) — NOTHING outside this crate references the capture items: the
//! binary wires [`DevCapturePlugin`] in-crate via `gdtf_app.rs`, and the config tests
//! are the in-crate `#[cfg(test)]` sibling. So every item here is `pub(crate)` (the
//! widest visibility anything reaches), keeping the binary `unreachable_pub`-clean
//! WITHOUT the `support_item!` flip.

use std::path::PathBuf;

use bevy::{
    prelude::*,
    render::view::window::screenshot::{Screenshot, ScreenshotCaptured, save_to_disk},
};

use crate::states::BattleScapeState;

/// The `GDTF_CAPTURE_PATH` environment variable: the absolute path of the output
/// PNG. Setting it (in a `dev_capture` debug build) opts into the capture affordance.
const CAPTURE_PATH_ENV: &str = "GDTF_CAPTURE_PATH";

/// The `GDTF_CAPTURE_FRAME` environment variable: how many `BattleRunning` frames to
/// wait before capturing (parsed into a [`CaptureFrame`]).
const CAPTURE_FRAME_ENV: &str = "GDTF_CAPTURE_FRAME";

/// How many frames AFTER the battle is running to wait before capturing.
///
/// The capture system counts only frames spent in
/// [`BattleScapeState::BattleRunning`]; once its `Local` counter reaches this value it
/// fires the screenshot. Waiting a handful of frames lets Bevy's UI layout flush so the
/// captured HUD is settled rather than mid-layout.
///
/// A named newtype over `u32` (no-bare-types). `pub(crate)`: referenced only by the
/// in-crate wiring + config tests.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deref)]
pub(crate) struct CaptureFrame(u32);

impl CaptureFrame {
    /// The default wait: 15 `BattleRunning` frames, enough for the UI layout to flush.
    pub(crate) const DEFAULT: Self = Self(15);

    /// Read the [`CaptureFrame`] from the [`CAPTURE_FRAME_ENV`] (`GDTF_CAPTURE_FRAME`)
    /// environment variable, falling back to [`CaptureFrame::DEFAULT`] when the variable
    /// is unset, empty, or not a valid `u32`. Never panics — a bad value silently uses
    /// the default.
    ///
    /// Pure (no `World`); delegates the parse to [`CaptureFrame::parse`] so the config
    /// tests can exercise the SAME logic without mutating the process-global env var.
    #[must_use]
    pub(crate) fn from_env() -> Self {
        Self::parse(std::env::var(CAPTURE_FRAME_ENV).ok().as_deref())
    }

    /// Parse a raw env-var value into a [`CaptureFrame`], falling back to
    /// [`CaptureFrame::DEFAULT`] when the value is absent, empty / whitespace, or not a
    /// valid `u32`. The pure core of [`CaptureFrame::from_env`], factored out so the
    /// config tests drive the REAL parse path with injected values (no env mutation).
    #[must_use]
    pub(crate) fn parse(value: Option<&str>) -> Self {
        value
            .and_then(|raw| raw.trim().parse::<u32>().ok())
            .map_or(Self::DEFAULT, Self)
    }
}

impl Default for CaptureFrame {
    /// The wiring default: [`CaptureFrame::DEFAULT`].
    fn default() -> Self {
        Self::DEFAULT
    }
}

/// Whether the DEV capture affordance is enabled for this process, and where it writes.
///
/// Reads the [`CAPTURE_PATH_ENV`] (`GDTF_CAPTURE_PATH`) environment variable and returns
/// the configured output path when it is set to a non-empty value; `None` (the
/// affordance stays inert) when the variable is unset or empty. This is the env-var half
/// of the gate; the `cfg!(all(debug_assertions, feature = "dev_capture"))` half lives at
/// the [`GdtfApp`](crate::GdtfApp) wiring site, so a release / default build never even
/// compiles the affordance in.
///
/// The path is framework plumbing handed straight to
/// [`save_to_disk`](bevy::render::view::window::screenshot::save_to_disk) — not a domain
/// value — so the no-bare-types rule does not apply to it.
///
/// Pure (no `World`, no side effects) so the GUI path can be reasoned about without
/// launching. Delegates the gate to [`parse_capture_path`] so the config tests can
/// exercise the SAME emptiness/trim logic without mutating the process-global env var.
/// `pub(crate)`.
#[must_use]
pub(crate) fn capture_path() -> Option<PathBuf> {
    parse_capture_path(std::env::var(CAPTURE_PATH_ENV).ok().as_deref())
}

/// Apply the capture-path gate to a raw env-var value: `Some(path)` when it is set to a
/// non-empty (trimmed) value, `None` (affordance inert) when absent, empty, or all
/// whitespace. The pure core of [`capture_path`], factored out so the config tests drive
/// the REAL gate with injected values (no env mutation). `pub(crate)`.
#[must_use]
pub(crate) fn parse_capture_path(value: Option<&str>) -> Option<PathBuf> {
    value
        .map(|raw| raw.trim().to_owned())
        .filter(|trimmed| !trimmed.is_empty())
        .map(PathBuf::from)
}

/// The resolved capture configuration: where to write and how long to wait.
///
/// Held by [`DevCapturePlugin`] when the affordance is enabled, and inserted as a
/// [`Resource`] so [`capture_when_ready`] can read both fields. Framework-plumbing
/// config (a path + a [`CaptureFrame`] newtype), not a domain value, so the
/// no-bare-types rule applies only to its `frame` field (which is the newtype).
/// `pub(crate)`: a purely internal resource, never re-exported.
#[derive(Resource, Debug, Clone)]
pub(crate) struct CaptureConfig {
    /// Absolute path of the output PNG, handed to
    /// [`save_to_disk`](bevy::render::view::window::screenshot::save_to_disk).
    path:  PathBuf,
    /// How many `BattleRunning` frames to wait before capturing.
    frame: CaptureFrame,
}

/// The DEV-ONLY screenshot / visual-QA affordance plugin (GTW-297).
///
/// Wired into [`GdtfApp`](crate::GdtfApp) only under `cfg!(all(debug_assertions, feature
/// = "dev_capture"))`. On `build` it consults its config ([`Self::config`]); when an
/// output path is present it logs a one-line `dev-capture: ON (dev)` and registers
/// [`capture_when_ready`] in `Update`, gated on [`BattleScapeState::BattleRunning`].
/// When no path is configured it registers nothing — the affordance is fully inert,
/// exactly like a build without the plugin. `pub(crate)`: named only by the in-crate
/// wiring + config tests.
pub(crate) struct DevCapturePlugin {
    /// The capture configuration, captured once at construction so `build` is a pure
    /// function of it. `None` means the affordance is inert.
    config: Option<CaptureConfig>,
}

impl DevCapturePlugin {
    /// Construct the affordance, reading its env-var gate ([`capture_path`]) and the
    /// [`CaptureFrame`] wait ([`CaptureFrame::from_env`]).
    ///
    /// This is the constructor [`GdtfApp`](crate::GdtfApp) uses under
    /// `cfg!(all(debug_assertions, feature = "dev_capture"))`: `GDTF_CAPTURE_PATH`
    /// decides whether the affordance activates, so the plugin is inert on a normal
    /// launch (the variable unset).
    #[must_use]
    pub(crate) fn from_env() -> Self {
        Self {
            config: capture_path().map(|path| CaptureConfig {
                path,
                frame: CaptureFrame::from_env(),
            }),
        }
    }

    /// Whether this affordance instance will activate on `build` (a path is configured).
    ///
    /// Test-only inherent surface (the config tests assert the gate without touching a
    /// process-global env var). `#[cfg(test)]` so the binary never compiles it (keeping
    /// it `dead_code`-clean).
    #[cfg(test)]
    #[must_use]
    pub(crate) const fn enabled(&self) -> bool {
        self.config.is_some()
    }

    /// The configured [`CaptureFrame`] wait, if the affordance is enabled.
    ///
    /// Test-only inherent surface (the config tests assert the default / parsed value).
    /// `#[cfg(test)]` so the binary never compiles it.
    #[cfg(test)]
    #[must_use]
    pub(crate) fn capture_frame(&self) -> Option<CaptureFrame> {
        self.config.as_ref().map(|config| config.frame)
    }
}

impl Default for DevCapturePlugin {
    /// The wiring default: read the env-var gate.
    fn default() -> Self {
        Self::from_env()
    }
}

impl Plugin for DevCapturePlugin {
    fn build(&self, app: &mut App) {
        let Some(config) = self.config.clone() else {
            // Inert: register nothing. The app runs normally and never captures.
            return;
        };
        info!("dev-capture: ON (dev)");
        app.insert_resource(config).add_systems(
            Update,
            capture_when_ready.run_if(in_state(BattleScapeState::BattleRunning)),
        );
    }
}

/// Captures the rendered battlescape frame to disk once the battle has been running for
/// [`CaptureFrame`] frames, then exits the app.
///
/// Runs in `Update`, gated `run_if(in_state(BattleScapeState::BattleRunning))`, so its
/// [`Local<u32>`] counter increments ONLY while the battle is on screen (it counts
/// frames since the battle rendered, not total app frames). Deliberately does NOT read
/// `Res<FrameCount>` (Bevy issue #24035 hang) — the `Local` is the frame source.
///
/// At the target frame it spawns a [`Screenshot::primary_window`] entity with an
/// observer that, when the frame is captured, calls
/// [`save_to_disk`](bevy::render::view::window::screenshot::save_to_disk) to write the
/// PNG and then writes `AppExit::Success`. The exit is written ONLY from inside that
/// observer (the `ci_testing` pattern, mitigating the macOS `AppExit` hang — Bevy issue
/// #23313). The capture is one-shot: once the counter passes the target the system does
/// nothing further, and the app is exiting anyway.
///
/// Param-only (`bevy-traps.md` #7): [`Commands`] + [`Res`]`<`[`CaptureConfig`]`>` + a
/// [`Local<u32>`] — no `&mut World`. The actual capture needs a real render device, so
/// this is verified by RUNNING the app (the orchestrator), NOT in a headless test.
fn capture_when_ready(
    mut commands: Commands,
    config: Res<CaptureConfig>,
    mut frames_in_battle: Local<u32>,
) {
    *frames_in_battle += 1;
    if *frames_in_battle != *config.frame {
        // Not yet (or already fired on an earlier frame): wait. Using `!=` rather than
        // `>=` keeps the spawn to the single target frame.
        return;
    }
    let path = config.path.clone();
    commands.spawn(Screenshot::primary_window()).observe(
        move |captured: On<ScreenshotCaptured>, mut exit: MessageWriter<AppExit>| {
            // Save the captured frame, then quit from INSIDE the observer (the
            // macOS-safe ci_testing exit pattern — bevy-traps.md #4 / issue #23313).
            save_to_disk(&path)(captured);
            exit.write(AppExit::Success);
        },
    );
}
