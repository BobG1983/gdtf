//! The DEV-ONLY QA affordance stack — everything that exists so a developer, a
//! coding agent, or the orchestrator's post-gate QA can drive, script, and capture
//! the app unattended. None of it is shipping behavior.
//!
//! This module OWNS the dev-QA harnesses (GTW-632); `crate::app` is purely the
//! [`GdtfApp`](crate::GdtfApp) wrapper. The members:
//!
//! - [`auto_battle`] — the `GDTF_AUTOBATTLE` auto-enter-battle drive (GTW-223).
//! - `capture` — the `GDTF_CAPTURE_*` screenshot / in-engine visual-QA affordance
//!   (GTW-297/GTW-306; see its module doc for the exact QA invocation).
//! - `drive` — the env-scripted battle-script triggers (`GDTF_FIRE_AT_FRAME` /
//!   `GDTF_FALL_AT_FRAME` / `GDTF_FIRE_MODE`): drive affordances over the REAL sim
//!   paths, not screenshot logic, so they live beside `capture`, not inside it.
//! - `capture_exit` — the ONE game-side capture EXIT (`poll_then_quit`) the
//!   per-scene capture hooks chain (GTW-577 C5).
//! - [`DevAffordancesPlugin`] — the ONE aggregate plugin owner
//!   [`GdtfApp`](crate::GdtfApp) wires; every cfg gate lives inside it.
//!
//! The per-scene capture HOOKS (the gang-editor / procgen-visualizer /
//! loading-screen `capture.rs` files) are scene-local drive + env glue over the
//! `gdtf_screenshot` primitives and deliberately STAY with their scenes
//! (GTW-577 P9) — they are consumers of this module's `capture_exit`, not members.

pub(crate) mod auto_battle;

// The DEV-ONLY screenshot / visual-QA affordance (GTW-297). Double-gated: only compiled
// under the opt-in `dev_capture` feature, and only wired in under `debug_assertions`.
#[cfg(all(debug_assertions, feature = "dev_capture"))]
pub(crate) mod capture;

// The ONE game-side capture EXIT (GTW-577 C5) the gang-editor and procgen-visualizer
// capture hooks chain after `gdtf_screenshot::settle_then_capture` — PNG-on-disk (or
// poll-cap) → `RunningState::Quit`, never a direct `AppExit`. Gated exactly like its
// two consumers; `pub(crate)` so the crate-root test-support ledger can name
// `poll_then_quit` for the headless pin test.
#[cfg(all(debug_assertions, feature = "dev_capture"))]
pub(crate) mod capture_exit;

// The env-scripted battle-script drive triggers (GTW-306/GTW-529). Gated exactly like
// `capture`: the capture plugin registers them and the shared env resolution lives in
// `capture::resolve`.
#[cfg(all(debug_assertions, feature = "dev_capture"))]
pub(crate) mod drive;

mod plugin;
pub(crate) use plugin::DevAffordancesPlugin;
