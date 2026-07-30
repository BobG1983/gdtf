//! Tests for the editor's screenshot capture pump (GTW-880).
//!
//! - [`aim`] — the pre-spawn consistency check (GTW-922): an offscreen capture whose target the
//!   editor's UI camera is not rendering into is refused with a typed reply, in both the
//!   window-targeted and the wrong-scale-factor cases, while a capture with no present path at
//!   all is spawned as before.
//! - [`ordering`] — the pump's target read runs AFTER the present path's retarget write
//!   (GTW-922): the real present plugin and the real pump in ONE app, with a shot whose settle
//!   expires on the frame the retarget fires, so the frame the ordering decides is the frame
//!   under test.
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

mod aim;
mod ordering;
mod pump;
mod source;
mod support;
