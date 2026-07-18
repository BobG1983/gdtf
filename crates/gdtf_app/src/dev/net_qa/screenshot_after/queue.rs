//! [`AfterShotQueue`] — the frame-delay countdown for an accepted `ScreenshotAfter`
//! (GTW-749, the T15 child).
//!
//! An intent that queues successfully does not fire its capture immediately: it waits
//! here, counting down once per frame, until its `frame_delay` elapses — then
//! [`fire_due`](AfterShotQueue::fire_due) hands it to the T7 pump's own
//! [`spawn_capture`](super::super::screenshot::spawn_capture), tagged
//! [`ReplyKind::After`](super::super::screenshot::ReplyKind::After) so the eventual
//! `Saved` / `TimedOut` reply rides the `ScreenshotAfter` wire envelope.

use bevy::prelude::*;
use gdtf_qa_protocol::ids::{FrameDelay, ShotName};

use super::super::{
    channel::Responder,
    screenshot::{
        CaptureSink, InFlightShots, QaShotDir, ReplyKind, ShotPollBudget, ShotSequence,
        spawn_capture,
    },
};

/// Frames remaining before a queued `ScreenshotAfter` capture fires.
///
/// Private-inner countdown newtype over `u32` (no-bare-types), mirroring the
/// `PollFramesLeft` / `FrameDeadline`-style countdowns already in this module family:
/// [`tick`](Self::tick) spends one frame and reports whether the capture is due NOW.
#[derive(Debug, Clone, Copy)]
struct AfterShotCountdown(u32);

/// The outcome of ticking an [`AfterShotCountdown`] — a typed alternative to a bare
/// `bool`.
enum CountdownTick {
    /// Still waiting — keep counting down next frame.
    Waiting,
    /// The countdown has elapsed — fire the capture now.
    Due,
}

impl AfterShotCountdown {
    /// Start a countdown of `delay` frames.
    fn new(delay: FrameDelay) -> Self {
        Self(*delay)
    }

    /// Spend one frame of the countdown, reporting whether the capture is due.
    ///
    /// Mirrors the module family's other countdowns' shape: `Due` fires on the tick
    /// that OBSERVES zero (never decrementing past it), so a `frame_delay` of `0` fires
    /// on the very first tick after the entry is queued.
    const fn tick(&mut self) -> CountdownTick {
        if self.0 == 0 {
            return CountdownTick::Due;
        }
        self.0 -= 1;
        CountdownTick::Waiting
    }
}

/// One accepted `ScreenshotAfter` counting down to its fire frame: the optional file
/// stem, the reply channel, and the countdown itself.
struct PendingAfterShot {
    /// The screenshot's file stem, or `None` for a server-chosen name.
    name:      Option<ShotName>,
    /// The reply channel back to the client.
    responder: Responder,
    /// Frames remaining before the capture fires.
    countdown: AfterShotCountdown,
}

/// The set of `ScreenshotAfter` captures counting down to their fire frame — a
/// [`Resource`] [`claim_screenshot_after`](super::claim_screenshot_after) pushes into and
/// [`tick_after_shots`](super::tick_after_shots) ticks + fires every frame (GTW-749).
#[derive(Resource, Default)]
pub(in crate::dev::net_qa) struct AfterShotQueue(Vec<PendingAfterShot>);

impl AfterShotQueue {
    /// Queue an accepted `ScreenshotAfter`'s capture to fire once `delay` more frames
    /// have ticked.
    pub(in crate::dev::net_qa) fn push(
        &mut self,
        delay: FrameDelay,
        name: Option<ShotName>,
        responder: Responder,
    ) {
        self.0.push(PendingAfterShot {
            name,
            responder,
            countdown: AfterShotCountdown::new(delay),
        });
    }

    /// Tick every queued capture; fire the real capture (via the shared T7
    /// [`spawn_capture`]) for any whose countdown just elapsed, keeping the rest for the
    /// next frame.
    pub(in crate::dev::net_qa) fn fire_due(
        &mut self,
        in_flight: &mut InFlightShots,
        budget: ShotPollBudget,
        dir: &QaShotDir,
        sequence: &mut ShotSequence,
        commands: &mut Commands,
    ) {
        if self.0.is_empty() {
            return;
        }
        let mut kept = Vec::with_capacity(self.0.len());
        for mut pending in self.0.drain(..) {
            match pending.countdown.tick() {
                CountdownTick::Due => {
                    spawn_capture(
                        pending.name.as_ref(),
                        pending.responder,
                        ReplyKind::After,
                        CaptureSink {
                            in_flight: &mut *in_flight,
                            budget,
                            dir,
                            sequence: &mut *sequence,
                            commands: &mut *commands,
                        },
                    );
                }
                CountdownTick::Waiting => kept.push(pending),
            }
        }
        self.0 = kept;
    }
}
