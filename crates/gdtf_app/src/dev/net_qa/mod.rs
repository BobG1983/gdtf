//! The DEV-ONLY QA network control channel — the GTW-694 architecture's T3 (GTW-736).
//!
//! A loopback-only (`Ipv4Addr::LOCALHOST`) TCP listener + request router a coding-agent
//! QA harness drives, speaking the bevy-free wire contract (`gdtf_qa_protocol`). NONE of
//! it is shipping behavior: the whole module compiles ONLY under
//! `cfg(all(debug_assertions, feature = "net_qa"))` (its wiring site in
//! [`crate::dev::plugin`] applies that double gate — it opens a listener), and even then
//! it is inert until `GDTF_NET_QA` is set.
//!
//! ## Members (one concern per file, per module-layout)
//!
//! - [`config`] — the typed transport config ([`NetQaPort`](config::NetQaPort) /
//!   [`NetIoTimeout`](config::NetIoTimeout)) + the server-identity constants.
//! - [`env`] — the `GDTF_NET_QA` / `GDTF_NET_QA_PORT` gates.
//! - [`channel`] — the listener → router request/response channel plumbing.
//! - [`listener`] — the loopback `std::thread` TCP transport (one client at a time,
//!   [`Busy`](gdtf_qa_protocol::envelope::QaError::Busy) on a second, two-sided timeouts).
//! - [`pending`] — the typed pending queues + the frame-deadline sweep.
//! - [`router`] — the always-on request router.
//! - [`convert`] — the wildcard-free [`NetIntent`](gdtf_qa_protocol::intent::NetIntent)
//!   classification (T4).
//! - [`resolve`] — the inject path's `SystemParam` bundles + fail-closed token/fire-mode
//!   resolvers + offer gate (T4).
//! - [`inject`] — the [`apply_injects`](inject::apply_injects) pump that wires injected
//!   intents into the same public input queues the local surfaces use (T4).
//! - [`snapshot`] — the on-demand battle-state view service that answers `GetBattleState`
//!   with a curated [`BattleView`](gdtf_qa_protocol::view::BattleView) read post-Simulate (T5).
//! - [`events`] — the outbox that answers `GetOutput` by projecting the GTW-727 act log
//!   onto the curated wire [`NetEvent`](gdtf_qa_protocol::events::NetEvent) stream (T6).
//! - [`present`] — the DEV-ONLY offscreen-capture present path (GTW-764): retargets the world
//!   and UI cameras to an offscreen image the render graph writes every tick, and blits that
//!   image back to the window, so the T7 pump captures pixels independent of window focus or
//!   occlusion (a backgrounded macOS window's swapchain reads back BLACK).
//! - [`screenshot`] — the deferred capture pump that answers `TakeScreenshot` by capturing
//!   the real presenter frame and replying only after the confined PNG lands on disk (T7).
//! - [`screenshot_after`] — the frame-exact deferred capture that answers `ScreenshotAfter`
//!   by injecting its embedded intent through the T4 path, then firing the T7 capture
//!   pipeline once its `frame_delay` countdown elapses (T15).
//! - [`start_battle`] — the navigation consumer that answers `StartBattle` by producing the
//!   SAME start-battle request the menu's Battlescape button produces (T9).
//! - [`stepper`] — the DEV procgen stepper-drive dispatch that answers `StepperControl` by
//!   writing the wire command into the SAME latch the egui panel's Next/Auto/Skip buttons
//!   write, serviceable only while a live `StagedProcgen` drive is in flight (GTW-766).
//! - [`activate_menu`] — the menu-item activation consumer that answers `ActivateMenuItem`
//!   by raising the SAME focus-activation message an `Enter` keypress raises for the token'd
//!   menu item, fail-closed on a stale token (GTW-787).
//! - [`focus_control`] — the focus-drive consumer that answers `FocusControl` by writing the
//!   SAME navigate message an arrow key writes and emitting a REAL `Enter` keypress at the
//!   focused control, so ANY focus-navigable screen is drivable, fail-closed on a stale or
//!   unlisted token (GTW-802).
//! - [`key_tap`] — the shared real-keypress emitter both the raw-input inject arm and the
//!   focus-drive activation write through.
//! - [`plugin`] — the [`NetQaPlugin`] registration (`from_env` / `with_channels`).

mod activate_menu;
mod channel;
mod config;
mod convert;
mod env;
mod events;
mod focus_control;
mod inject;
mod key_tap;
mod listener;
mod pending;
mod plugin;
mod present;
mod resolve;
mod router;
mod screenshot;
mod screenshot_after;
mod snapshot;
mod start_battle;
mod stepper;

// `NetQaPlugin` is the item the binary consumes (via the dev aggregate plugin,
// `crate::dev::plugin`), so it re-exports in BOTH configurations at the `test-support`
// visibility flip the item itself uses (`support_item!` in `plugin`): `pub` under
// `test-support` (the `test_support` ledger needs it), `pub(crate)` otherwise —
// `unreachable_pub`-clean either way.
crate::support_use!(plugin::NetQaPlugin;);

// The transport + router TEST surface is consumed ONLY through the `test_support` ledger
// (the GTW-736 integration suite). The binary never names these, so re-exporting them in
// a non-`test-support` build would be an unused `pub(crate) use`; gate the re-export to
// the same feature, `pub` because the ledger needs it.
#[cfg(feature = "test-support")]
pub use channel::{IncomingRequest, Responder};
#[cfg(feature = "test-support")]
pub use config::{NET_QA_PROTOCOL_VERSION, NetIoTimeout, NetQaPort};
// GTW-727: the ONE availability predicate, exposed so the input-gate suite can assert the
// catch-up gating directly. It is the same function the router's accept/reject and the
// advertised `available` list both call, so a test against it cannot drift from either.
#[cfg(feature = "test-support")]
pub use router::request_available as request_available_for;
// GTW-740: the T7 pump's confinement-directory + poll-budget config Resources, exposed for
// the integration test to inject a temp directory + a tiny budget. Widened through
// `screenshot`'s own `support_use!` re-export; gated to `test-support` like the rest.
#[cfg(feature = "test-support")]
pub use screenshot::{QaShotDir, ShotPollBudget};
// GTW-742: the shipped-situation name the T9 `StartBattle` consumer accepts, exposed so the
// T9 integration test names a VALID situation ref without hard-coding the literal. Gated to
// `test-support` like the rest.
#[cfg(feature = "test-support")]
pub use start_battle::SHIPPED_SITUATION;
