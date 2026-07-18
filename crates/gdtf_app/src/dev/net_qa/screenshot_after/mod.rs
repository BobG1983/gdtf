//! The frame-exact deferred screenshot — the GTW-694 architecture's T15 (GTW-749).
//!
//! [`ScreenshotAfter`](gdtf_qa_protocol::envelope::QaRequest::ScreenshotAfter) composes the
//! T4 inject path and the T7 screenshot pump rather than adding a third, parallel one:
//! [`claim_screenshot_after`] injects the request's embedded intent through the EXACT SAME
//! classify-and-push [`receipt_for`](super::inject::receipt_for) the T4 `apply_injects` pump
//! calls, so a rejected intent (`NotOffered` / `UnknownEntity` / …) answers immediately with
//! its typed rejection and NO capture is ever attempted — never a shot of the wrong moment.
//! An accepted intent's capture is deferred into [`AfterShotQueue`], which
//! [`tick_after_shots`] counts down every frame; once the `frame_delay` elapses it fires the
//! REAL capture through the T7 pump's own [`spawn_capture`](super::screenshot::spawn_capture)
//! — the identical confinement + delete-before-spawn + poll-until-decodable pipeline a plain
//! `TakeScreenshot` uses, so the reply lands only once the PNG verifiably exists on disk (or
//! the poll budget elapses).
//!
//! ## Why the game counts the frames, not the client
//!
//! A request/response round-trip cannot land on a specific frame — the game must count
//! `frame_delay` frames itself after the intent queues. This is what lets a QA client catch a
//! TRANSIENT effect (a muzzle flash, an impact flash) mid-animation instead of settled: it
//! injects the act, tells the game how many frames to wait, and the game captures on the
//! frame the animation is actually mid-flight.
//!
//! ## Members (one concern per file, per module-layout)
//!
//! - [`queue`] — [`AfterShotQueue`], the frame-delay countdown state + the fire-when-due
//!   hand-off into the T7 pump.
//! - [`drive`] — [`claim_screenshot_after`] (the same-frame claim + intent-inject) and
//!   [`tick_after_shots`] (the unconditional per-frame countdown + fire).

mod drive;
mod queue;

pub(in crate::dev::net_qa) use drive::{claim_screenshot_after, tick_after_shots};
pub(in crate::dev::net_qa) use queue::AfterShotQueue;
