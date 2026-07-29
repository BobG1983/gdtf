//! Tests for the editor's screenshot capture pump (GTW-880).
//!
//! - [`support`] — the headless app carrying the REAL pump, the router-shaped enqueue, and the
//!   file-planting helpers that stand in for the GPU.
//! - [`pump`] — the pump behaviours the wire-level integration suite cannot reach: a STALE PNG
//!   already at the target path is deleted before the capture spawns and never served, an
//!   existing-but-UNDECODABLE file is rejected, and a PNG that lands AFTER the capture is
//!   spawned is reported saved without needing a GPU.
//! - [`source`] — which pixels a capture reads (GTW-917): each
//!   [`EditorShotSource`](super::config::EditorShotSource) arm maps to the render target its
//!   spawned `Screenshot` carries, and `NetQaEditorPlugin` installs the source the shipped
//!   editor captures through.

mod pump;
mod source;
mod support;
