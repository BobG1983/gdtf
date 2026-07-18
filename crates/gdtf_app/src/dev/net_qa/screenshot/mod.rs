//! The screenshot capture service — the GTW-694 architecture's T7 (GTW-740).
//!
//! Answers the routed
//! [`TakeScreenshot`](gdtf_qa_protocol::envelope::QaRequest::TakeScreenshot) requests the T3
//! router queues by capturing the REAL graphical presenter frame to a PNG and replying only
//! after that PNG verifiably lands on disk (or a frame budget elapses). Unlike the same-frame
//! T4 inject / T5 snapshot pumps, the reply is genuinely DEFERRED across frames — the GPU
//! readback cannot flush the file the instant it is requested.
//!
//! ## Members (one concern per file, per module-layout)
//!
//! - [`path`] — confines a wire [`ShotName`](gdtf_qa_protocol::ids::ShotName) to a `.png`
//!   path strictly under `target/qa_screenshots/`, then makes it per-capture UNIQUE
//!   (`<stem>_<n>.png`) so no two captures share a file to clobber.
//! - [`verify`] — the independent exists + non-empty + PNG-decodes check.
//! - [`pump`] — the [`drive_screenshots`](pump::drive_screenshots) pump (poll-before-claim,
//!   delete-before-spawn), its [`InFlightShots`](pump::InFlightShots) tracking, the poll
//!   budget / confinement directory / sequence config, and the [`spawn_capture`](pump::spawn_capture)
//!   pipeline the T15 `screenshot_after` child (GTW-749) shares to fire its own deferred
//!   captures through the SAME confinement + poll machinery (tagged
//!   [`ReplyKind::After`](pump::ReplyKind::After) instead of
//!   [`ReplyKind::Direct`](pump::ReplyKind::Direct)).

mod path;
mod pump;
mod verify;

#[cfg(test)]
mod test;

pub(in crate::dev::net_qa) use path::ShotSequence;
pub(in crate::dev::net_qa) use pump::{
    CaptureSink, InFlightShots, ReplyKind, drive_screenshots, spawn_capture,
};
// GTW-740: widened to the `test_support` visibility flip so the T7 integration test can
// inject a temp confinement directory + a tiny poll budget (the pump's own config Resources,
// exposed for test injection exactly like `NetIoTimeout` / `NetQaPort`). `pub` under
// `test-support`, `pub(crate)` otherwise — reachable by `plugin.rs` either way, and
// `unreachable_pub`-clean in both configurations.
crate::support_use!(path::QaShotDir;);
crate::support_use!(pump::ShotPollBudget;);
