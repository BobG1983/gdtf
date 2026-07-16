//! `gdtf_qa_protocol` — the bevy-free typed **wire contract** for the QA net layer
//! (GTW-734, QA-net T1; the adopted GTW-694 architecture §1.1 / §1.2).
//!
//! This crate is the SHARED vocabulary spoken between two future halves of the QA
//! control channel:
//!
//! - the game-side `net_qa` server (`gdtf_app/src/dev/net_qa`, T3) that owns the
//!   listener, the intent injection, the snapshot, and the screenshot, and
//! - the `bins/gdtf_qa_mcp` MCP bridge (T8) that a language-model harness drives.
//!
//! It compiles WITHOUT the Bevy engine on purpose: it depends only on `serde` (the
//! wire encoding), `ron` (the compact on-wire text), and `bevy_derive` (the `Deref`
//! derive alone — a proc-macro crate, no engine). Both halves can therefore link it
//! without pulling wgpu / winit / the ECS. Everything here is DATA plus one PURE
//! framing codec — no I/O, no transport, no async.
//!
//! # Module map
//!
//! - [`ids`] — the newtype id / handle vocabulary: entity [`tokens`](ids::token),
//!   the coordinate wire types ([`ids::cell`]), and the misc handles
//!   ([`ids::handle`]). Every one is a private-inner newtype with a derived `Deref`
//!   (no-bare-types rules 1–5).
//! - [`intent`] — [`NetIntent`](intent::NetIntent), the wire mirror of the
//!   `gdtf_battle_input` act vocabulary (the classic + contextual acts a QA client
//!   can inject).
//! - [`view`] — the curated read-model DTOs ([`BattleView`](view::BattleView) and
//!   friends): independent serde types, never a leak of a sim type.
//! - [`events`] — [`NetEvent`](events::NetEvent) + [`EventBatch`](events::EventBatch),
//!   the curated combat-event outbox subset.
//! - [`envelope`] — [`QaRequest`](envelope::QaRequest) /
//!   [`QaResponse`](envelope::QaResponse), the top-level request/response protocol.
//! - [`framing`] — the length-prefixed frame codec as pure functions (encode + an
//!   incremental, split-read-tolerant decoder).
//!
//! # The one guarantee
//!
//! Every public type round-trips through compact RON identically
//! (`ron::ser::to_string` → `ron::de::from_str`), proven by the per-module
//! round-trip suites — the property both halves rely on.

pub mod envelope;
pub mod events;
pub mod framing;
pub mod ids;
pub mod intent;
pub mod view;

#[cfg(test)]
mod test_support;
