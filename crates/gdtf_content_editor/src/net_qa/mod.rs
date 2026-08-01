//! The DEV-ONLY QA network control channel for the EDITOR (GTW-804, child 1b of GTW-786).
//!
//! The editor half of the GTW-694 QA harness: a loopback-only (`Ipv4Addr::LOCALHOST`) TCP
//! listener a coding-agent QA client drives, speaking the bevy-free wire contract
//! ([`gdtf_qa_protocol`]). It runs on the SHARED transport ([`gdtf_net_qa_transport`],
//! GTW-803) — the game and the editor move bytes through ONE codepath — and this module is
//! the editor's HOST side of it: the activation gates, the request drain, and the capture
//! pump.
//!
//! NONE of it is shipping behavior. The whole module compiles ONLY under
//! `cfg(all(debug_assertions, feature = "net_qa"))` (the wiring site in `crate::app` applies
//! the same double gate — it opens a listener), and even then it is inert until
//! `GDTF_EDITOR_NET_QA` is set truthy.
//!
//! ## Why the editor has its own env vars and its own default port
//!
//! The game's `GDTF_NET_QA` / `GDTF_NET_QA_PORT` / port `7616` are the GAME's activation
//! policy, deliberately left in `gdtf_app` when GTW-803 lifted the transport. Two hosts
//! sharing one switch and one port would fight for the socket: launching the editor with the
//! game's variable set would make one of them fail to bind. So the editor mirrors the
//! CONVENTION — one truthy enable variable, one port variable, loopback-only, one client —
//! with its OWN names and its OWN default port (see [`config`] and [`env`]).
//!
//! ## Members (one concern per file, per module-layout)
//!
//! - [`config`] — this server's identity constants + its default listen port.
//! - [`env`] — the `GDTF_EDITOR_NET_QA` / `GDTF_EDITOR_NET_QA_PORT` gates.
//! - [`schedule`] — the [`EditorNetQaSystems`] set the request drain runs in.
//! - [`router`] — the request drain: it answers `Catalogue` with this host's (still empty)
//!   command list and every `Run` `Unknown`, because the editor's own command set is the
//!   next editor ticket.
//! - [`screenshot`] — the capture pump: settle, capture, and reply only once the PNG has
//!   landed on disk (GTW-880).
//! - [`present`] — the offscreen capture target the running editor captures through: the
//!   window-sized `COPY_SRC` image, the egui-camera retarget, and the present camera that blits
//!   it back to the window (GTW-918).
//! - [`plugin`] — the [`NetQaEditorPlugin`] registration.

mod config;
mod env;
mod plugin;
mod present;
mod router;
mod schedule;
mod screenshot;

pub use config::EDITOR_QA_SERVER_NAME;
pub use plugin::NetQaEditorPlugin;
pub use schedule::EditorNetQaSystems;
pub use screenshot::{
    EditorQaShotDir, EditorScreenshotPayload, EditorShotPollBudget, EditorShotSettle,
    EditorShotSource,
};
