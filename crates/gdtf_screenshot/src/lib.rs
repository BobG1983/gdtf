//! `gdtf_screenshot` — reusable in-engine self-capture primitives for dev / agent visual QA
//! (GTW-510, a child of GTW-17).
//!
//! Wraps Bevy 0.19's built-in screenshot render path
//! (`bevy::render::view::window::screenshot::{Screenshot, ScreenshotCaptured, save_to_disk}`) into
//! a small set of reusable pieces so the game AND the content editor can capture a REAL rendered
//! frame to a STABLE, agent-readable PNG — which a QA pass then `Read`s to assert on the ACTUAL
//! rendered layout, not just headless entity existence. This closes the view-ticket visual-QA gap
//! that shipped the mangled editor (GTW-509).
//!
//! This crate is DEV / DEBUG infrastructure. It carries no gate of its own — the consuming crate
//! adds it behind a debug/dev feature gate (`cfg!(all(debug_assertions, feature = "dev_capture"))`)
//! or a `cfg!(debug_assertions)` keybind gate, exactly as the procgen visualizer is dev-gated, so
//! it is compiled OUT of release (matching the `gdtf_app` `capture` module discipline).
//!
//! ## What it exposes
//!
//! - [`CapturePath`] — the STABLE output-path newtype; [`parse_shot_path`] is the ONE pure env-value
//!   gate every consumer delegates to; [`timestamped_path`] computes the keybind default under
//!   `target/screenshots/`.
//! - [`SettleFrames`] / [`PollCap`] — the settle-before-read timing newtypes (documented defaults:
//!   egui-safe 30, simple-scene 15).
//! - [`CaptureProgress`] + [`settle_then_capture`] / [`poll_then_exit`] / [`reset_progress`] — the
//!   generic settle-then-spawn-`Screenshot` + poll-then-exit systems each scene's hook can delegate
//!   to instead of re-implementing the counter + spawn + poll boilerplate.
//! - [`ScreenshotCapturePlugin`] — the env-var / programmatic capture-then-exit plugin for a plain
//!   binary with no game state machine (the content editor's simplest wiring; the reference a QA
//!   harness drives).
//! - [`KeyboardCapturePlugin`] — the interactive debug-keybind trigger (F10 → capture, no exit).
//!
//! ## Three triggers, all dev-gated by the consumer
//!
//! 1. **Env-var capture-then-exit** (unattended QA / CI-able): set the opt-in env var to a path;
//!    [`ScreenshotCapturePlugin::from_env`] captures after settle and exits. The scene hooks in
//!    `gdtf_app` (gang editor / procgen visualizer / loading screen, since GTW-577) keep their
//!    own env vars + drive but delegate the path gate to [`parse_shot_path`] and the
//!    settle-then-`Screenshot` spawn to [`settle_then_capture`]; their EXIT is the game's own
//!    `RunningState::Quit` cascade (`gdtf_app`'s `poll_then_quit`), NOT [`poll_then_exit`] —
//!    and the loading-screen hook deliberately does not exit at all (capture-and-continue).
//! 2. **Debug keybind** (interactive): [`KeyboardCapturePlugin`] on F10, no exit.
//! 3. **Programmatic** (harness): [`ScreenshotCapturePlugin::with_path`] / inserting a
//!    [`CapturePath`] resource + advancing the app drives the same pipeline.

/// The debug-keybind capture trigger.
pub mod keybind;
/// The [`CapturePath`] output-path newtype + pure path helpers.
pub mod path;
/// The env-var / programmatic capture-then-exit plugin.
pub mod plugin;
/// The settle-before-read timing newtypes.
pub mod settle;
/// The generic settle-then-capture + poll-then-exit systems + progress state.
pub mod trigger;

pub use keybind::{CaptureTag, KeyboardCapturePlugin};
pub use path::{CapturePath, parse_shot_path, timestamped_path};
pub use plugin::ScreenshotCapturePlugin;
pub use settle::{PollCap, SettleFrames};
pub use trigger::{
    CaptureProgress, FrameCount, ShotRequested, poll_then_exit, reset_progress, settle_then_capture,
};
