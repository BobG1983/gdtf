//! Tests for the editor's screenshot capture pump (GTW-880).
//!
//! - [`support`] — the headless app carrying the REAL pump, the router-shaped enqueue, and the
//!   file-planting helpers that stand in for the GPU.
//! - [`pump`] — the pump behaviours the wire-level integration suite cannot reach: a STALE PNG
//!   already at the target path is deleted before the capture spawns and never served, an
//!   existing-but-UNDECODABLE file is rejected, and a PNG that lands AFTER the capture is
//!   spawned is reported saved without needing a GPU.

mod pump;
mod support;
