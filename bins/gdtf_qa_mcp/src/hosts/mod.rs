//! The two QA hosts this MCP bridge manages — the game and the content editor (GTW-808).
//!
//! One `gdtf_qa_mcp` binary drives both child processes (the user's 2026-07-24 ruling on
//! GTW-786: a single dual-target host, not a second server). This module is where "which
//! host" is a value rather than a branch:
//!
//! - [`host`] — the [`QaHost`] enum and everything that differs per host: the default
//!   port, the default launch recipe, the timing config.
//! - [`set`] — the [`HostPair`] (one host's link + lifecycle) and the [`HostSet`] that
//!   resolves a [`QaHost`] to its pair.

pub mod host;
pub mod set;

pub use host::QaHost;
pub use set::{HostPair, HostSet};
