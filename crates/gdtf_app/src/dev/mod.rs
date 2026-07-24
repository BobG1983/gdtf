//! The DEV-ONLY QA affordance stack — everything that exists so a developer, a
//! coding agent, or the orchestrator's post-gate QA can drive, script, and capture
//! the app unattended. None of it is shipping behavior.
//!
//! This module OWNS the dev-QA harnesses (GTW-632); `crate::app` is purely the
//! [`GdtfApp`](crate::GdtfApp) wrapper. The members:
//!
//! - [`net_qa`] — the loopback QA network control channel (GTW-736 onward): intent
//!   injection, battle-state snapshots, screenshots (immediate and frame-exact deferred),
//!   event drains, and start-battle navigation, all over one wire contract
//!   (`gdtf_qa_protocol`). This is the ONE capture / drive path now — it replaced the
//!   `GDTF_AUTOBATTLE` auto-enter-battle drive and the `GDTF_CAPTURE_*` /
//!   `GDTF_FIRE_AT_FRAME` / `GDTF_FIRE_MODE` env-var capture/drive rig outright (GTW-749),
//!   not as a fallback beside them.
//! - [`DevAffordancesPlugin`] — the ONE aggregate plugin owner
//!   [`GdtfApp`](crate::GdtfApp) wires; every cfg gate lives inside it.
//!
//! The per-scene capture HOOKS (the loading-screen `capture.rs`, currently the only
//! remaining one) are scene-local drive + env glue over the `gdtf_screenshot`
//! primitives and deliberately STAY with their scenes (GTW-577 P9). The GTW-434
//! procgen-visualizer hook's game-side capture-EXIT sibling (`capture_exit`,
//! `poll_then_quit`) was removed with it in GTW-655 — the loading-screen hook never
//! chained it (capture-and-continue, no exit) and no other scene did either.

// The DEV-ONLY procgen load-time stepper (GTW-655): an egui Next/Auto/Skip overlay over the
// sim's staged procgen driver, replacing the GTW-434 menu-invoked procgen-visualizer scene.
// Compiled ONLY under the opt-in `dev_tools` feature (which pulls in `bevy_egui`) — a
// release artifact, and a normal `dev_tools`-less dev build, never link egui.
#[cfg(feature = "dev_tools")]
pub(crate) mod procgen_stepper;

// The DEV-ONLY UI-stack coexistence proof-of-concept (GTW-819): one `bevy_ui` button and one
// egui button alive together in `AppState::Running`, answering whether the two UI stacks can
// coexist at all before the GTW-796 comparison epic builds on the assumption. Compiled ONLY
// under the opt-in `dev_tools` feature (the one that pulls in `bevy_egui`), like the stepper.
#[cfg(feature = "dev_tools")]
pub(crate) mod ui_coexistence;

// The DEV-ONLY QA network control channel (GTW-736): a loopback TCP listener + request
// router a coding-agent QA harness drives. Double-gated: compiled only under the opt-in
// `net_qa` feature (which pulls in the bevy-free wire contract + the self-capture crate),
// and only wired in under `debug_assertions` (it opens a listener). A release artifact
// never links it.
#[cfg(all(debug_assertions, feature = "net_qa"))]
pub(crate) mod net_qa;

mod plugin;
pub(crate) use plugin::DevAffordancesPlugin;
