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
//! - [`DevAffordancesPlugin`] — the ONE aggregate plugin owner
//!   [`GdtfApp`](crate::GdtfApp) wires; every cfg gate lives inside it.
//!
//! The per-scene capture HOOKS (the loading-screen `capture.rs`, currently the only
//! remaining one) are scene-local drive + env glue over the `gdtf_screenshot`
//! primitives and deliberately STAY with their scenes (GTW-577 P9). The GTW-434
//! procgen-visualizer hook's game-side capture-EXIT sibling (`capture_exit`,
//! `poll_then_quit`) was removed with it in GTW-655 — the loading-screen hook never
//! chained it (capture-and-continue, no exit) and no other scene did either.

pub(crate) mod auto_battle;

// The DEV-ONLY screenshot / visual-QA affordance (GTW-297). Double-gated: only compiled
// under the opt-in `dev_capture` feature, and only wired in under `debug_assertions`.
#[cfg(all(debug_assertions, feature = "dev_capture"))]
pub(crate) mod capture;

// The env-scripted battle-script drive triggers (GTW-306/GTW-529). Gated exactly like
// `capture`: the capture plugin registers them and the shared env resolution lives in
// `capture::resolve`.
#[cfg(all(debug_assertions, feature = "dev_capture"))]
pub(crate) mod drive;

// The DEV-ONLY procgen load-time stepper (GTW-655): an egui Next/Auto/Skip overlay over the
// sim's staged procgen driver, replacing the GTW-434 menu-invoked procgen-visualizer scene.
// Compiled ONLY under the opt-in `dev_tools` feature (which pulls in `bevy_egui`) — a
// release artifact, and a normal `dev_tools`-less dev build, never link egui.
#[cfg(feature = "dev_tools")]
pub(crate) mod procgen_stepper;

// The DEV-ONLY QA network control channel (GTW-736): a loopback TCP listener + request
// router a coding-agent QA harness drives. Double-gated exactly like `capture`/`drive` —
// compiled only under the opt-in `net_qa` feature (which pulls in the bevy-free wire
// contract + the self-capture crate), and only wired in under `debug_assertions` (it
// opens a listener). A release artifact never links it.
#[cfg(all(debug_assertions, feature = "net_qa"))]
pub(crate) mod net_qa;

mod plugin;
pub(crate) use plugin::DevAffordancesPlugin;
