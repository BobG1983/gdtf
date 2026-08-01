//! The always-on request drain (GTW-736; the command arms are GTW-942; cut to the command
//! layer by GTW-943).
//!
//! [`route_requests`] drains the [`NetInbox`](gdtf_net_qa_transport::NetInbox) every frame
//! in the [`InputSystems::Gather`](gdtf_battle_input::InputSystems) band and dispatches each
//! [`QaRequest`](gdtf_qa_protocol::message::QaRequest). There are only two arms that do
//! work: `Catalogue` answers from the frame's facts on the spot, and `Run` resolves the
//! named command against this host's own set and parks an admitted call for that command's
//! decode step.
//!
//! # There is exactly ONE drain
//!
//! [`route_requests`] is the only system in the game that reads
//! [`NetInbox`](gdtf_net_qa_transport::NetInbox), and that is a requirement rather than a
//! coincidence. `NetInbox::drain()` is `rx.try_iter().collect()` — it takes EVERYTHING in
//! the channel — so a second router reading the same resource in the same frame would
//! swallow the first one's requests, nondeterministically, depending on which ran first.
//! The property is pinned by `crates/gdtf_app/tests/net_qa/command_set.rs`.
//!
//! `Hello` is NOT answered here (GTW-940). The listener thread negotiates it against this
//! host's [`hello_facts()`](super::config::hello_facts) before the inbox is reached, and
//! refuses every other request `NotNegotiated` until it has. Its match arm answers
//! `Malformed`: a handshake that reached the inbox got past the one place that answers it,
//! so the frame is wrong for this connection.
//!
//! ## Members (one concern per file, per module-layout)
//!
//! - [`route`] — the drain itself.

pub(in crate::dev::net_qa) mod route;

pub(in crate::dev::net_qa) use route::route_requests;
