//! The DEV-ONLY QA network control channel — the GTW-694 architecture's T3 (GTW-736).
//!
//! A loopback-only (`Ipv4Addr::LOCALHOST`) TCP listener + request router a coding-agent
//! QA harness drives, speaking the bevy-free wire contract (`gdtf_qa_protocol`). NONE of
//! it is shipping behavior: the whole module compiles ONLY under
//! `cfg(all(debug_assertions, feature = "net_qa"))` (its wiring site in
//! [`crate::dev::plugin`] applies that double gate — it opens a listener, the
//! `dev_capture` strictness class), and even then it is inert until `GDTF_NET_QA` is set.
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
//! - [`plugin`] — the [`NetQaPlugin`] registration (`from_env` / `with_channels`).

mod channel;
mod config;
mod convert;
mod env;
mod inject;
mod listener;
mod pending;
mod plugin;
mod resolve;
mod router;
mod snapshot;

// `NetQaPlugin` is the item the binary consumes (via the dev aggregate plugin,
// `crate::dev::plugin`), so it re-exports in BOTH configurations at the `test-support`
// visibility flip the item itself uses (`support_item!` in `plugin`): `pub` under
// `test-support` (the `test_support` ledger needs it), `pub(crate)` otherwise —
// `unreachable_pub`-clean either way. Mirrors `crate::dev::auto_battle`.
crate::support_use!(plugin::NetQaPlugin;);

// The transport + router TEST surface is consumed ONLY through the `test_support` ledger
// (the GTW-736 integration suite). The binary never names these, so re-exporting them in
// a non-`test-support` build would be an unused `pub(crate) use`; gate the re-export to
// the same feature, `pub` because the ledger needs it. Mirrors the `auto_battle` split.
#[cfg(feature = "test-support")]
pub use channel::{IncomingRequest, Responder};
#[cfg(feature = "test-support")]
pub use config::{NET_QA_PROTOCOL_VERSION, NetIoTimeout, NetQaPort};
