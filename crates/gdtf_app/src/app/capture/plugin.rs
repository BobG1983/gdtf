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
//!   `GDTF_CAPTURE_FRAME=<n>` (default [`CaptureFrame::DEFAULT`]): captures ONE frame
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
//! target in [`CaptureFrames`] it spawns a [`Screenshot::primary_window`] entity with an
//! observer that saves the frame to that frame's PNG path; on the LAST target frame the
//! observer also writes `AppExit::Success`.
//!
//! `AppExit::Success` is written ONLY from inside the
//! [`ScreenshotCaptured`](bevy::render::view::window::screenshot::ScreenshotCaptured)
//! observer (the `ci_testing` pattern) to mitigate the macOS `AppExit` hang (Bevy issue
//! #23313, not fixed in 0.18.1). If a future Bevy still hangs there, the documented
//! fallback is to despawn the `PrimaryWindow` entity to take winit's native exit path.
//!
//! The actual screenshot capture needs a real render device, so it CANNOT be
//! headless-tested — it is verified by RUNNING the app (the orchestrator does so, then
//! `Read`s the PNGs). The headless tests cover the [`DevCapturePlugin::from_env`] config
//! logic (path gate, [`CaptureFrame`] / [`CaptureFrames`] / [`FireAtFrame`] parse) AND
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

use std::path::{Path, PathBuf};

use bevy::{
    prelude::*,
    render::view::window::screenshot::{Screenshot, ScreenshotCaptured, save_to_disk},
};
use gdtf_battle_input::{SelectedFireMode, SelectedShooter};
use gdtf_battle_sim::{
    Cell, Faction, FireMode, FireModeSpec, Level, ModeKind, PlayerFaction, Position,
    acts::FireRequested,
};

use crate::states::BattleScapeState;

/// The `GDTF_CAPTURE_PATH` environment variable: the absolute path of the output
/// PNG. Setting it (in a `dev_capture` debug build) opts into the capture affordance.
const CAPTURE_PATH_ENV: &str = "GDTF_CAPTURE_PATH";

/// The `GDTF_CAPTURE_FRAME` environment variable: how many `BattleRunning` frames to
/// wait before capturing a SINGLE frame (parsed into a [`CaptureFrame`]).
const CAPTURE_FRAME_ENV: &str = "GDTF_CAPTURE_FRAME";

/// The `GDTF_CAPTURE_FRAMES` environment variable: a comma-separated list of
/// `BattleRunning` frames to capture in ONE run (parsed into a [`CaptureFrames`]). When
/// set it wins over [`CAPTURE_FRAME_ENV`]; one PNG is written per listed frame.
const CAPTURE_FRAMES_ENV: &str = "GDTF_CAPTURE_FRAMES";

/// The `GDTF_FIRE_AT_FRAME` environment variable: the `BattleRunning` frame at which the
/// selected player ganger fires at the nearest enemy via the real fire path (parsed into
/// a [`FireAtFrame`]).
const FIRE_AT_FRAME_ENV: &str = "GDTF_FIRE_AT_FRAME";

/// The `GDTF_FIRE_MODE` environment variable: which fire MODE the dev fire-trigger shoots
/// in — `single` / `burst` / `full` (parsed into a [`FireModeOverride`]). Unset leaves the
/// trigger using the resident [`SelectedFireMode`]. Used by the GTW-306 FX capture to drive
/// a multi-round (burst / full-auto) volley so the staggered projectiles are observable.
const FIRE_MODE_ENV: &str = "GDTF_FIRE_MODE";

/// How many frames AFTER the battle is running to wait before capturing a single frame.
///
/// The capture system counts only frames spent in
/// [`BattleScapeState::BattleRunning`]; once its `Local` counter reaches this value it
/// fires the screenshot. Waiting a handful of frames lets Bevy's UI layout flush so the
/// captured HUD is settled rather than mid-layout.
///
/// A named newtype over `u32` (no-bare-types). `pub(crate)`: referenced only by the
/// in-crate wiring + config tests.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Deref)]
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

/// The ordered, de-duplicated set of `BattleRunning` frames to capture in one run.
///
/// A named newtype over `Vec<CaptureFrame>` (no-bare-types: the capture schedule is a
/// domain value) holding a SORTED, DEDUPED, NON-EMPTY list. The single-frame
/// `GDTF_CAPTURE_FRAME` path is just the one-element case, so the capture system handles
/// both uniformly. `pub(crate)`: referenced only by the in-crate wiring + config tests.
#[derive(Debug, Clone, PartialEq, Eq, Deref)]
pub(crate) struct CaptureFrames(Vec<CaptureFrame>);

impl CaptureFrames {
    /// Read the [`CaptureFrames`] from the env vars: the comma-list
    /// [`CAPTURE_FRAMES_ENV`] (`GDTF_CAPTURE_FRAMES`) when it parses to ≥1 frame,
    /// otherwise the single [`CaptureFrame::from_env`] ([`CAPTURE_FRAME_ENV`], itself
    /// defaulting). Pure (no `World`); delegates to [`CaptureFrames::parse`] so the
    /// config tests drive the SAME logic without mutating the process-global env var.
    #[must_use]
    pub(crate) fn from_env() -> Self {
        Self::parse(
            std::env::var(CAPTURE_FRAMES_ENV).ok().as_deref(),
            CaptureFrame::from_env(),
        )
    }

    /// Parse a raw `GDTF_CAPTURE_FRAMES` comma-list into a [`CaptureFrames`], falling
    /// back to the single `fallback` frame when the list is absent, empty, or holds no
    /// valid `u32` entry. Splits on `,`, trims each entry, keeps the valid `u32`s, then
    /// SORTS + DEDUPES so the capture system fires each frame once in order. The pure
    /// core of [`CaptureFrames::from_env`]; never panics.
    #[must_use]
    pub(crate) fn parse(list: Option<&str>, fallback: CaptureFrame) -> Self {
        let mut frames: Vec<CaptureFrame> = list
            .into_iter()
            .flat_map(|raw| raw.split(','))
            .filter_map(|entry| entry.trim().parse::<u32>().ok().map(CaptureFrame))
            .collect();
        frames.sort_unstable();
        frames.dedup();
        if frames.is_empty() {
            // No valid list entries -> the single-frame schedule (the `GDTF_CAPTURE_FRAME`
            // / default path stays working).
            frames.push(fallback);
        }
        Self(frames)
    }

    /// The last (highest) frame in the schedule — the one whose capture writes
    /// `AppExit::Success`. Infallible: the list is non-empty by construction.
    #[must_use]
    fn last_frame(&self) -> CaptureFrame {
        // `copied().max()` over a non-empty sorted list; the `unwrap_or` is a structural
        // safety net (never taken) that keeps the no-`unwrap` rule satisfied.
        self.0
            .iter()
            .copied()
            .max()
            .unwrap_or(CaptureFrame::DEFAULT)
    }

    /// Whether this schedule is the single-frame case (exactly one target frame). In
    /// that case the capture writes the exact `GDTF_CAPTURE_PATH` (no per-frame suffix),
    /// preserving the GTW-297 single-frame behavior.
    #[must_use]
    const fn is_single(&self) -> bool {
        self.0.len() == 1
    }
}

/// The frame at which the dev fire-trigger fires the selected player ganger.
///
/// A named newtype over `u32` (no-bare-types). `pub(crate)`: referenced only by the
/// in-crate wiring + config tests + the [`trigger_fire_at_frame`] system.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deref)]
pub(crate) struct FireAtFrame(u32);

impl FireAtFrame {
    /// Build a fire-trigger frame from its raw frame index. Test-only inherent surface
    /// (the production parse builds one through the tuple constructor in-module; the test
    /// constructs through the newtype, not the private field). `#[cfg(test)]` so the
    /// binary stays `dead_code`-clean.
    #[cfg(test)]
    #[must_use]
    pub(crate) const fn new(frame: u32) -> Self {
        Self(frame)
    }

    /// Read the optional [`FireAtFrame`] from the [`FIRE_AT_FRAME_ENV`]
    /// (`GDTF_FIRE_AT_FRAME`) env var. `None` (the trigger stays inert) when the var is
    /// unset, empty, or not a valid `u32`. Pure (no `World`); delegates to
    /// [`FireAtFrame::parse`].
    #[must_use]
    pub(crate) fn from_env() -> Option<Self> {
        Self::parse(std::env::var(FIRE_AT_FRAME_ENV).ok().as_deref())
    }

    /// Parse a raw env-var value into an optional [`FireAtFrame`]: `Some` for a valid
    /// `u32`, `None` (trigger inert) for an absent / empty / non-numeric value. The pure
    /// core of [`FireAtFrame::from_env`]; never panics.
    #[must_use]
    pub(crate) fn parse(value: Option<&str>) -> Option<Self> {
        value
            .and_then(|raw| raw.trim().parse::<u32>().ok())
            .map(Self)
    }
}

/// An optional dev override for the fire-trigger's fire MODE — which authored
/// [`ModeKind`] the triggered shot fires in (`Single` / `Burst` / `Full`).
///
/// When set (`GDTF_FIRE_MODE`), [`trigger_fire_at_frame`] picks the matching
/// [`FireModeSpec`] off the selected shooter's authored [`FireMode`] selector and fires in
/// THAT mode (so a `full` override produces a multi-round volley the FX stagger spreads
/// out). When unset the trigger uses the resident [`SelectedFireMode`] unchanged.
///
/// A named newtype over the closed [`ModeKind`] (no-bare-types). `pub(crate)`: referenced
/// only by the in-crate wiring + config tests + [`trigger_fire_at_frame`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deref)]
pub(crate) struct FireModeOverride(ModeKind);

impl FireModeOverride {
    /// Build a fire-mode override from a [`ModeKind`]. Test-only inherent surface (the
    /// production path constructs it through [`FireModeOverride::parse`]); `#[cfg(test)]`
    /// so the binary stays `dead_code`-clean.
    #[cfg(test)]
    #[must_use]
    pub(crate) const fn new(kind: ModeKind) -> Self {
        Self(kind)
    }

    /// Read the optional [`FireModeOverride`] from the [`FIRE_MODE_ENV`] (`GDTF_FIRE_MODE`)
    /// env var. `None` (the trigger keeps the resident mode) when the var is unset, empty,
    /// or not a recognised mode name. Pure (no `World`); delegates to
    /// [`FireModeOverride::parse`].
    #[must_use]
    pub(crate) fn from_env() -> Option<Self> {
        Self::parse(std::env::var(FIRE_MODE_ENV).ok().as_deref())
    }

    /// Parse a raw env-var value into an optional [`FireModeOverride`]: `Some` for a
    /// recognised, case-insensitive mode name (`single` / `burst` / `full` / `full-auto`),
    /// `None` for an absent / empty / unrecognised value. The pure core of
    /// [`FireModeOverride::from_env`]; never panics.
    #[must_use]
    pub(crate) fn parse(value: Option<&str>) -> Option<Self> {
        let kind = match value?.trim().to_ascii_lowercase().as_str() {
            "single" => ModeKind::Single,
            "burst" => ModeKind::Burst,
            "full" | "full-auto" | "fullauto" => ModeKind::Full,
            _ => return None,
        };
        Some(Self(kind))
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

/// Insert a `.fNN` frame tag before a capture path's extension, e.g. `out.png` at frame
/// `12` -> `out.f12.png` (multi-frame). With no extension the tag is appended:
/// `shot` -> `shot.f12`. Used only on the multi-frame path; the single-frame path writes
/// the exact base path.
///
/// Pure path plumbing (no `World`) so the multi-frame naming is unit-testable.
/// `pub(crate)`.
#[must_use]
pub(crate) fn frame_path(base: &Path, frame: CaptureFrame) -> PathBuf {
    let mut tagged = base.to_path_buf();
    let stem = base
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("capture");
    let tagged_name = match base.extension().and_then(|e| e.to_str()) {
        Some(ext) => format!("{stem}.f{}.{ext}", *frame),
        None => format!("{stem}.f{}", *frame),
    };
    tagged.set_file_name(tagged_name);
    tagged
}

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
    /// multi-frame path each frame is suffixed via [`frame_path`].
    path:   PathBuf,
    /// Which `BattleRunning` frames to capture (one PNG per frame).
    frames: CaptureFrames,
}

/// The resolved fire-trigger configuration: the frame to fire on.
///
/// Held by [`DevCapturePlugin`] when the fire sub-affordance is enabled, inserted as a
/// [`Resource`] so [`trigger_fire_at_frame`] can read it. `pub(crate)`: internal only.
#[derive(Resource, Debug, Clone, Copy)]
pub(crate) struct FireConfig {
    /// The `BattleRunning` frame at which the selected player ganger fires.
    frame: FireAtFrame,
    /// An optional fire-MODE override (`GDTF_FIRE_MODE`): when set, the trigger fires in
    /// this authored [`ModeKind`] (read off the shooter's [`FireMode`]) rather than the
    /// resident [`SelectedFireMode`] — the FX-capture path uses `Full` for a staggered
    /// multi-round volley.
    mode:  Option<FireModeOverride>,
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
}

impl Default for DevCapturePlugin {
    /// The wiring default: read the env-var gates.
    fn default() -> Self {
        Self::from_env()
    }
}

impl Plugin for DevCapturePlugin {
    fn build(&self, app: &mut App) {
        if self.capture.is_none() && self.fire.is_none() {
            // Inert: register nothing. The app runs normally and never captures / fires.
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
    }
}

/// Captures the rendered battlescape frame(s) to disk on each scheduled
/// [`BattleRunning`](BattleScapeState::BattleRunning) frame, then exits the app after the
/// last.
///
/// Runs in `Update`, gated `run_if(in_state(BattleScapeState::BattleRunning))`, so its
/// [`Local<u32>`] counter increments ONLY while the battle is on screen (it counts
/// frames since the battle rendered, not total app frames). Deliberately does NOT read
/// `Res<FrameCount>` (Bevy issue #24035 hang) — the `Local` is the frame source.
///
/// On each frame whose count matches a [`CaptureFrames`] target it spawns a
/// [`Screenshot::primary_window`] entity with an observer that calls
/// [`save_to_disk`](bevy::render::view::window::screenshot::save_to_disk) to write that
/// frame's PNG (the exact path for the single-frame case, a `.fNN`-tagged path otherwise
/// — [`frame_path`]); the observer for the LAST target frame ALSO writes
/// `AppExit::Success`. The exit is written ONLY from inside that observer (the
/// `ci_testing` pattern, mitigating the macOS `AppExit` hang — Bevy issue #23313).
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
    let current = CaptureFrame(*frames_in_battle);
    if !config.frames.contains(&current) {
        // Not a scheduled frame: wait.
        return;
    }
    let is_last = current == config.frames.last_frame();
    let path = if config.frames.is_single() {
        // Single-frame: write the exact GDTF_CAPTURE_PATH (GTW-297 behavior preserved).
        config.path.clone()
    } else {
        // Multi-frame: one PNG per frame, `.fNN`-tagged.
        frame_path(&config.path, current)
    };
    commands.spawn(Screenshot::primary_window()).observe(
        move |captured: On<ScreenshotCaptured>, mut exit: MessageWriter<AppExit>| {
            // Save this frame, then (only on the last scheduled frame) quit from INSIDE
            // the observer (the macOS-safe ci_testing exit pattern — bevy-traps.md #4 /
            // issue #23313).
            save_to_disk(&path)(captured);
            if is_last {
                exit.write(AppExit::Success);
            }
        },
    );
}

/// At the configured [`FireAtFrame`] (counted in
/// [`BattleRunning`](BattleScapeState::BattleRunning) frames), makes the selected player
/// ganger fire at the nearest enemy via the REAL fire path.
///
/// This is the GTW-306 dev fire-trigger. It does NOT fake a shot: it writes a
/// [`FireRequested`](gdtf_battle_sim::acts::FireRequested) message — the exact message a
/// left-click over an enemy produces — so the sim's `dispatch_fire` resolves the volley
/// (TU spend, arc, hit roll, `ShotFired`), and the FX slices then render the projectile /
/// impact off `ShotFired`. The shooter is the auto-selected
/// [`SelectedShooter`](gdtf_battle_input::SelectedShooter) (a player-faction ganger), the
/// mode is the [`SelectedFireMode`](gdtf_battle_input::SelectedFireMode), and the target
/// is the nearest enemy ganger (a [`Faction`] `!=` [`PlayerFaction`]) by squared cell
/// distance.
///
/// Fires exactly once: it spends only while its [`Local<u32>`] counter equals the target
/// frame. A frame with no selection, no player faction, or no enemy in range is a no-op
/// (the shot simply does not fire — the affordance is best-effort dev tooling).
///
/// Param-only (`bevy-traps.md` #7): a [`MessageWriter<FireRequested>`], the
/// `Res<SelectedShooter>` / `Res<SelectedFireMode>` / `Option<Res<PlayerFaction>>` reads,
/// a read-only `Query<(Entity, &Faction, &Position)>`, and a [`Local<u32>`] — no
/// `&mut World`. `Option<Res<PlayerFaction>>` because that resource exists only inside
/// the battle window (`bevy-traps.md` #1).
///
/// `pub(crate)` so the headless test drives this REAL system directly (registered in
/// `Update` minus the unrelated `BattleScapeState` sub-state gate — the same "drive the
/// real system on its real schedule, minus unrelated state wiring" idiom the auto-battle
/// A1 test uses), asserting it emits one `FireRequested` at frame N.
pub(crate) fn trigger_fire_at_frame(
    mut fires: MessageWriter<FireRequested>,
    config: Res<FireConfig>,
    selected: Res<SelectedShooter>,
    fire_mode: Res<SelectedFireMode>,
    player: Option<Res<PlayerFaction>>,
    gangers: Query<(Entity, &Faction, &Position, Option<&FireMode>)>,
    mut frames_in_battle: Local<u32>,
) {
    *frames_in_battle += 1;
    if *frames_in_battle != *config.frame {
        // Not the trigger frame (or already fired): wait. `!=` keeps the fire to the one
        // target frame.
        return;
    }
    // The selected player ganger; bail (no-op) if nothing is selected.
    let Some(shooter) = **selected else {
        return;
    };
    let Some(player) = player else {
        return;
    };
    let player_faction = **player;
    // The shooter's own cell (to pick the NEAREST enemy) + its authored fire-mode selector
    // (consulted only when GDTF_FIRE_MODE overrides the mode).
    let Ok((_, _, shooter_pos, shooter_modes)) = gangers.get(shooter) else {
        return;
    };
    let shooter_cell = Cell::new(shooter_pos.x, shooter_pos.y);
    // The mode the shot fires in: the resident SelectedFireMode by default, or — when
    // GDTF_FIRE_MODE is set — the matching authored mode off the shooter's FireMode selector
    // (so a `full` override yields a multi-round volley the FX stagger can spread out). An
    // override naming a mode the weapon does not offer (or an unarmed shooter) falls back to
    // the resident mode.
    let mode = config
        .mode
        .and_then(|override_kind| fire_mode_spec(shooter_modes, *override_kind))
        .unwrap_or(**fire_mode);
    // The nearest enemy ganger (a faction != the player's) by squared cell distance.
    let Some((_, enemy_pos)) = gangers
        .iter()
        .filter(|(entity, faction, ..)| *entity != shooter && **faction != player_faction)
        .map(|(_, _, pos, _)| {
            let delta = Cell::new(pos.x, pos.y);
            let dx = delta.x - shooter_cell.x;
            let dy = delta.y - shooter_cell.y;
            (dx * dx + dy * dy, pos)
        })
        .min_by_key(|(dist_sq, _)| *dist_sq)
    else {
        return;
    };
    let target_cell = Cell::new(enemy_pos.x, enemy_pos.y);
    let target_level = Level::new(level_from_z(enemy_pos.z));
    fires.write(FireRequested::new(shooter, mode, target_cell, target_level));
}

/// The authored [`FireModeSpec`] for `kind` on a shooter's optional [`FireMode`] selector,
/// or [`None`] when the shooter is unarmed (no selector) or does not offer that mode.
///
/// Used by the dev fire-trigger's `GDTF_FIRE_MODE` override to fire in a specific authored
/// mode (e.g. `Full`) rather than the resident [`SelectedFireMode`]. Pure read-only lookup.
fn fire_mode_spec(modes: Option<&FireMode>, kind: ModeKind) -> Option<FireModeSpec> {
    modes?.iter().copied().find(|spec| spec.kind == kind)
}

/// Narrow a `(cell, level)` key's storey-index `z` (an `i32` in the inner `IVec3`) to the
/// [`Level`]'s `u8`, saturating into range — a non-negative storey index is small, so
/// this never realistically clamps. `as`-cast trips `cast_possible_truncation` (`-D`);
/// `u8::try_from` is the no-`unwrap` narrow, saturating an out-of-range value to a valid
/// storey rather than panicking.
fn level_from_z(z: i32) -> u8 {
    u8::try_from(z.clamp(0, i32::from(u8::MAX))).unwrap_or(0)
}
