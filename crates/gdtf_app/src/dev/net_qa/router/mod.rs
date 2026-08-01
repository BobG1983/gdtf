//! The always-on request router (GTW-736; the affordance advertisement is GTW-746; the
//! command arms are GTW-942).
//!
//! [`route_requests`] drains the [`NetInbox`](gdtf_net_qa_transport::NetInbox) every frame
//! in the [`InputSystems::Gather`](gdtf_battle_input::InputSystems) band and dispatches each
//! [`QaRequest`](gdtf_qa_protocol::envelope::QaRequest). One predicate,
//! [`request_available`], decides which requests the router services in the current state:
//!
//! - A request the router cannot service now is rejected at ROUTE time via
//!   `Option<Res<_>>` witnesses — never a panic on the missing resource (bevy-traps #1). The
//!   battle-dependent quartet (`Inject` / `GetBattleState` / `GetOutput` /
//!   `ScreenshotAfter`) arriving with no battle running is rejected `NoBattle`
//!   (`ScreenshotAfter` embeds an intent, so it is gated exactly like a bare `Inject`,
//!   GTW-749 — never a stale capture); the DEV `StepperControl` arriving with no live
//!   procgen-stepper drive is rejected `StepperInactive` (GTW-766).
//! - A serviceable request is then answered directly (`GetAppFlow`, `Catalogue`), resolved
//!   against this host's command set and parked for its own command (`Run`), or enqueued for
//!   its (later) consumer (`Inject` / `GetBattleState` / `GetOutput` / `TakeScreenshot` /
//!   `ScreenshotAfter` / `StartBattle` / `StepperControl`).
//!
//! The `GetAppFlow` answer reports the same set — [`available_requests`] filters
//! [`RequestKindNet::ALL`](gdtf_qa_protocol::view::RequestKindNet) through the SAME
//! [`request_available`] predicate — so what a QA client is told it may send and what the
//! router actually accepts are computed from one source and cannot disagree.
//!
//! The system is ALWAYS registered when the plugin is active; per-request behavior varies
//! but the system itself never blinks in and out.
//!
//! # There is exactly ONE drain
//!
//! [`route_requests`] is the only system in the game that reads
//! [`NetInbox`](gdtf_net_qa_transport::NetInbox), and that is a requirement rather than a
//! coincidence. `NetInbox::drain()` is `rx.try_iter().collect()` — it takes EVERYTHING in
//! the channel — so a second router reading the same resource in the same frame would
//! swallow the first one's requests, nondeterministically, depending on which ran first.
//! That is why the GTW-942 command layer WIDENED this system with `Catalogue` and `Run`
//! arms instead of registering a router of its own. The property is pinned by
//! `crates/gdtf_app/tests/net_qa/command_set.rs`.
//!
//! `Hello` is NOT in the dispatch (GTW-940). The listener thread negotiates it against this
//! host's [`hello_facts()`](super::config::hello_facts) before the inbox is reached, and
//! refuses every other request `NotNegotiated` until it has, so the duplicated
//! `answer_hello` this file and the editor's router each carried is gone. The kind stays
//! ADVERTISED as available — the server really does service it — and its match arm answers
//! `Malformed`: a handshake that reached the inbox got past the one place that answers it,
//! so the frame is wrong for this connection rather than a request this host refuses
//! (GTW-942).
//!
//! ## Members (one concern per file, per module-layout)
//!
//! - [`available`] — the one availability predicate, the advertised set it produces, and
//!   the accurate refusal reason.
//! - [`route`] — the drain itself.

pub(in crate::dev::net_qa) mod available;
pub(in crate::dev::net_qa) mod route;

pub(in crate::dev::net_qa) use available::available_requests;
// The GTW-727 input-gate suite asserts the catch-up gating against this exact function.
// `route` imports it from `available` directly, so this re-export exists ONLY to carry the
// name up to the `test_support` ledger — hence the feature gate (without it the import is
// unused) and the `pub` (a narrower one would cap the item and break the ledger, E0365).
#[cfg(feature = "test-support")]
pub use available::request_available;
pub(in crate::dev::net_qa) use route::{app_state_to_net, route_requests};
