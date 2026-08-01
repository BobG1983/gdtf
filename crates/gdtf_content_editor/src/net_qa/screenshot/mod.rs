//! The editor's screenshot capture service (GTW-880, child of GTW-786).
//!
//! Captures a REAL rendered frame to a PNG and answers only after that PNG verifiably lands
//! on disk (or a poll budget elapses). The reply is genuinely DEFERRED across frames — the
//! shell settles first, and the GPU readback cannot flush the file the instant it is
//! requested.
//!
//! Nothing on the wire fills the queue today (GTW-943 deleted the `TakeScreenshot` request
//! with the rest of the pre-command vocabulary); the editor's own capture command is what
//! fills it next.
//!
//! The deferred-reply machinery is the SHARED transport's, not a second copy: the router
//! pushes onto a [`PendingQueue`](gdtf_net_qa_transport::PendingQueue) and the plugin
//! registers [`sweep_pending`](gdtf_net_qa_transport::sweep_pending) for this payload type,
//! exactly as the game's T7 pump does.
//!
//! ## What the offscreen capture source stands in for, and what it does not (GTW-917, GTW-918)
//!
//! Every test that watches a capture actually LAND drives
//! [`EditorShotSource::Offscreen`](config::EditorShotSource::Offscreen) — an image a camera
//! renders into — because no test app has a window: a cargo test thread cannot create winit's
//! event loop on macOS, so there is no swapchain to read back.
//!
//! The offscreen source stands in for all of this, and these are genuinely covered: the
//! settle window before a capture is spawned, the delete-before-spawn purge of a stale file at
//! the target path, the across-frames disk poll loop, the PNG decode check, and the reply
//! deferred until the file verifiably landed. The suite drives from the pump's own queue, not
//! a socket — GTW-943 deleted the request that was the socket half, so everything from the
//! `PendingQueue` down is real and nothing routes onto it yet.
//!
//! It does NOT stand in for whether a REAL editor window's swapchain yields the egui shell
//! rather than a black frame — and in particular whether it does so when that window is not
//! visible. That is a property of the primary-window source alone, and per GTW-764 a
//! backgrounded, occluded or minimized macOS window reads back black. GTW-918 removed the
//! dependency on that: the running editor now renders its egui camera into an offscreen image
//! ([`super::present`]) and captures THAT, so window visibility no longer decides whether a
//! capture shows the shell. Reading the resulting pixels back from a real editor process is
//! what GTW-904 exists to do.
//!
//! What IS pinned here about the source choice is the mapping itself: `test/source.rs`
//! asserts each [`EditorShotSource`](config::EditorShotSource) arm spawns the render target it
//! names, and that `NetQaEditorPlugin` installs the source the shipped editor captures through.
//!
//! ## Members (one concern per file, per module-layout)
//!
//! - [`payload`] — the queued request's payload (the client's optional file stem).
//! - [`path`] — confines a wire [`ShotName`](gdtf_qa_protocol::ids::ShotName) to a `.png`
//!   path strictly under [`EditorQaShotDir`], then makes it per-capture UNIQUE
//!   (`<stem>_<n>.png`) so no two captures share a file to clobber.
//! - [`config`] — the settle window, the poll budget, and which pixels a capture reads.
//! - [`aim`] — the pre-spawn consistency check (GTW-922): an offscreen capture whose target the
//!   editor's UI camera is not rendering into is refused, with the mismatch named, instead of
//!   landing a blank PNG that reads as a screenshot of the editor.
//! - [`verify`] — the independent exists + non-empty + PNG-decodes check.
//! - [`spawn`] — which render target one capture reads, and the delete-before-spawn purge plus
//!   directory creation around its output file.
//! - [`pump`] — the [`drive_editor_screenshots`](pump::drive_editor_screenshots) pump
//!   (advance-before-claim, settle-then-capture, delete-before-spawn) and its
//!   [`EditorInFlightShots`](pump::EditorInFlightShots) tracking.
//! - `test` — pump tests for the mechanisms a fresh temp directory puts out of the wire
//!   suite's reach: the delete-before-spawn purge of a stale PNG, the rejection of an
//!   undecodable file, and the GPU-free landing case.

mod aim;
mod config;
mod path;
mod payload;
mod pump;
mod spawn;
mod verify;

#[cfg(test)]
mod test;

// The three tunables + the confinement directory carry the crate's `net_qa`-gated public
// surface (the crate root re-exports them) so the integration suite can pin a short settle,
// a small poll budget, a temp output directory and an offscreen capture source.
pub use config::{EditorShotPollBudget, EditorShotSettle, EditorShotSource};
pub use path::EditorQaShotDir;
pub(super) use path::EditorShotSequence;
pub use payload::EditorScreenshotPayload;
pub(super) use pump::{EditorInFlightShots, drive_editor_screenshots};
