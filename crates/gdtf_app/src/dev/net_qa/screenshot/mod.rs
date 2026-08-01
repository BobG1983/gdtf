//! The screenshot capture service — the GTW-694 architecture's T7 (GTW-740, retargeted onto
//! the command layer by GTW-943).
//!
//! Captures the REAL graphical presenter frame to a PNG and answers only after that PNG
//! verifiably lands on disk (or a frame budget elapses). The reply is genuinely DEFERRED
//! across frames — the GPU readback cannot flush the file the instant it is requested.
//!
//! # What fills the queue
//!
//! Nothing on the wire, today. The `TakeScreenshot` request that used to feed it went with
//! the rest of the pre-command vocabulary in GTW-943, and the `capture` rider on a
//! [`Run`](gdtf_qa_protocol::message::QaRequest::Run) is what feeds it next (the C2 ticket).
//! The pump, its confinement, its uniqueness sequence and its verification are all still
//! under test through [`ScreenshotPayload`](payload::ScreenshotPayload), which the suite
//! pushes onto the REAL [`PendingQueue`](gdtf_net_qa_transport::PendingQueue).
//!
//! ## Members (one concern per file, per module-layout)
//!
//! - [`payload`] — what a queued capture carries: the optional caller-chosen file stem.
//! - [`path`] — confines a wire [`ShotName`](gdtf_qa_protocol::ids::ShotName) to a `.png`
//!   path strictly under `target/qa_screenshots/`, then makes it per-capture UNIQUE
//!   (`<stem>_<n>.png`) so no two captures share a file to clobber.
//! - [`verify`] — the independent exists + non-empty + PNG-decodes check.
//! - [`pump`] — the [`drive_screenshots`](pump::drive_screenshots) pump (poll-before-claim,
//!   delete-before-spawn), its [`InFlightShots`](pump::InFlightShots) tracking, and the poll
//!   budget / confinement directory / sequence config.

mod path;
mod payload;
mod pump;
mod verify;

#[cfg(test)]
mod test;

pub(in crate::dev::net_qa) use path::ShotSequence;
pub(in crate::dev::net_qa) use pump::{InFlightShots, drive_screenshots};

// GTW-740: widened to the `test_support` visibility flip so the capture suite can inject a
// temp confinement directory + a tiny poll budget, and enqueue a capture through the real
// queue. `pub` under `test-support`, `pub(crate)` otherwise — reachable by `plugin.rs`
// either way, and `unreachable_pub`-clean in both configurations.
crate::support_use!(path::QaShotDir;);
crate::support_use!(payload::ScreenshotPayload;);
crate::support_use!(pump::ShotPollBudget;);
