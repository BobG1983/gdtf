//! The host-agnostic DEV QA network transport (GTW-803, lifted from GTW-736's
//! `gdtf_app/src/dev/net_qa/`).
//!
//! A loopback-only (`Ipv4Addr::LOCALHOST`) TCP listener that serves ONE client at a time,
//! hands each decoded request to its host over an [`mpsc`](std::sync::mpsc) channel as an
//! [`IncomingRequest`], and frames the host's reply back. The host-side half is the
//! [`NetInbox`] resource it drains, plus a typed [`PendingQueue`] per request kind and the
//! [`sweep_pending`] pump that answers
//! [`Timeout`](gdtf_qa_protocol::message::QaError::Timeout) on an entry nothing claimed —
//! so a client is never left hanging.
//!
//! ## What this crate deliberately does NOT hold
//!
//! - **Routing and request meaning.** Which request is serviceable, what a payload means,
//!   and every consumer that answers one stay with the host (for the game, in
//!   `gdtf_app::dev::net_qa`). This crate moves bytes and correlates replies.
//! - **Framing.** The `u32` BE length prefix, the encoder and the frame decoder live in
//!   [`gdtf_qa_protocol::framing`] and are bevy-free already. The listener CALLS that
//!   codec; it never reimplements or re-exports it.
//! - **The activation gates.** A host decides whether to open a listener at all. The game
//!   double-gates its wiring site on `cfg(all(debug_assertions, feature = "net_qa"))` and
//!   then reads its own `GDTF_NET_QA` / `GDTF_NET_QA_PORT` env vars; another host picks its
//!   own switch and its own port. This crate declares no `cfg` and no env var of its own —
//!   what it does enforce, in code, is loopback-only binding and a single client.
//!
//! ## Members (one concern per file, per module-layout)
//!
//! (The modules themselves are private — every item they own is re-exported flat from this
//! root, so a consumer imports `gdtf_net_qa_transport::PendingQueue` and the internal split
//! stays free to move. The names below are those private modules, not links.)
//!
//! - `config` — the typed transport configuration ([`NetQaPort`], [`NetIoTimeout`]) and the
//!   default socket timeout.
//! - `channel` — the listener → host request/response channel plumbing.
//! - `listener` — the loopback bind, the accept loop with its one-client gate, and the
//!   per-connection serve loop.
//! - `pending` — the typed pending queue, its per-entry frame deadline, and the sweep.

mod channel;
mod config;
mod listener;
mod pending;

pub use channel::{IncomingRequest, NetInbox, Responder};
pub use config::{DEFAULT_IO_TIMEOUT, NetIoTimeout, NetQaPort};
pub use listener::{bind_listener, run_listener};
pub use pending::{PendingQueue, sweep_pending};
