//! `gdtf_qa_protocol` — the bevy-free typed **wire contract** for the QA net layer
//! (GTW-734; cut down to the command layer by GTW-943).
//!
//! This crate is the SHARED vocabulary spoken between the two halves of the QA control
//! channel:
//!
//! - the HOSTS — the game's `net_qa` server (`gdtf_app/src/dev/net_qa`) and the content
//!   editor's (`gdtf_content_editor/src/net_qa`) — each of which publishes its own command
//!   set, and
//! - the `bins/gdtf_qa_mcp` MCP courier a language-model harness drives.
//!
//! It compiles WITHOUT the Bevy engine on purpose: it depends only on `serde` (the wire
//! encoding), `ron` (the compact on-wire text), and `bevy_derive` (the `Deref` derive alone
//! — a proc-macro crate, no engine). Both halves can therefore link it without pulling
//! wgpu / winit / the ECS. Everything here is DATA plus one PURE framing codec — no I/O, no
//! transport, no async.
//!
//! # The shape of the wire
//!
//! Three requests, four responses, five errors, and that is all of it. What a HOST can do
//! is not a variant here: it is a command, and a command is a row in a
//! [`CommandCatalogue`](command::CommandCatalogue) plus a name inside a
//! [`Run`](message::QaRequest::Run). So a host can grow from one command to sixty without
//! moving [`ProtocolVersion::CURRENT`](message::ProtocolVersion::CURRENT), without a
//! courier change, and without an MCP reconnect. `src/message/test/freeze.rs` is what holds
//! that line.
//!
//! # Module map
//!
//! - [`message`] — [`QaRequest`](message::QaRequest) / [`QaResponse`](message::QaResponse),
//!   the handshake, and the error vocabulary: the outer wrapper of every exchange.
//! - [`command`] — the command vocabulary (GTW-939): a host's live
//!   [`CommandCatalogue`](command::CommandCatalogue), the JSON argument / reply bodies a
//!   call carries, the derived schemas a catalogue row publishes, whether a command can run
//!   right now, and what running it produced.
//! - [`ids`] — the wire ids the shapes above name directly: the grid coordinates a host's
//!   command arguments embed, and the capture file stem a
//!   [`CaptureRider`](command::CaptureRider) carries. Every one is a private-inner newtype
//!   with a derived `Deref` (no-bare-types rules 1–5).
//! - [`framing`] — the length-prefixed frame codec as pure functions (encode + an
//!   incremental, split-read-tolerant decoder).
//!
//! # The one guarantee
//!
//! Every public type round-trips through compact RON identically
//! (`ron::ser::to_string` → `ron::de::from_str`), proven by the per-module round-trip
//! suites — the property both halves rely on.

pub mod command;
pub mod framing;
pub mod ids;
pub mod message;

#[cfg(test)]
mod test_support;
