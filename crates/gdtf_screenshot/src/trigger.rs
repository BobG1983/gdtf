//! The generic capture systems: settle-then-spawn a `Screenshot` request, and poll-then-exit once
//! the PNG has landed.
//!
//! This is the shared boilerplate every scene's capture hook used to re-implement (a
//! `Local`/counter settle gate + the `commands.spawn(Screenshot::primary_window()).observe(
//! save_to_disk(..))` call + a disk poll before exit). The counters live in a
//! [`CaptureProgress`] `Resource` built from named newtypes ([`FrameCount`] / [`ShotRequested`]),
//! so a consumer can drive + inspect the pipeline headlessly (the pipeline-wiring test asserts
//! [`ShotRequested`] flips even where a GPU-less run cannot produce the PNG).
//!
//! The EXIT after the PNG lands is split by consumer: a plain binary with no game state machine
//! (the content editor) uses [`poll_then_exit`] (writes `AppExit`); a game-state-machine consumer
//! (the `gdtf_app` scenes) instead rides its own `RunningState::Quit` shutdown cascade (writing
//! `AppExit` from an observer does not reliably terminate winit on macOS — Bevy #23313), so it does
//! NOT use [`poll_then_exit`].

use bevy::{
    prelude::*,
    render::view::window::screenshot::{Screenshot, save_to_disk},
};

use crate::{
    path::CapturePath,
    settle::{PollCap, SettleFrames},
};

/// A count of frames elapsed since the capture became eligible.
///
/// A named newtype over `u32` (no-bare-types): the elapsed-frame value is a domain counter, not a
/// bare `u32`. The inner is PRIVATE — advanced only through [`FrameCount::tick`], read through
/// [`Deref`].
#[derive(Clone, Copy, Default, Debug, Deref)]
pub struct FrameCount(u32);

impl FrameCount {
    /// Advance the counter by one frame.
    pub const fn tick(&mut self) {
        self.0 += 1;
    }
}

/// Whether this run's screenshot has already been requested (so it fires exactly once).
///
/// A named flag newtype (no-bare-types): the "request fired" state is a domain value, not a bare
/// `bool`. The inner is PRIVATE — set via [`ShotRequested::mark`], read via [`ShotRequested::is_marked`].
#[derive(Clone, Copy, Default, Debug, Deref)]
pub struct ShotRequested(bool);

impl ShotRequested {
    /// Mark the screenshot as requested.
    #[must_use]
    pub const fn mark() -> Self {
        Self(true)
    }

    /// Whether the screenshot has already been requested this run.
    #[must_use]
    pub const fn is_marked(self) -> bool {
        self.0
    }
}

/// Per-run capture progress: how many frames have elapsed since the capture became eligible, and
/// whether the screenshot has been requested yet.
///
/// Inserted (via [`Default`]) by [`ScreenshotCapturePlugin`](crate::ScreenshotCapturePlugin) when
/// the affordance is active; read + advanced by [`settle_then_capture`] and read by
/// [`poll_then_exit`]. Both leaves are named newtypes ([`FrameCount`] / [`ShotRequested`]).
#[derive(Resource, Default, Debug)]
pub struct CaptureProgress {
    /// Frames elapsed since the capture became eligible (reset on the owning scene's `OnEnter`).
    frames:    FrameCount,
    /// Whether the screenshot has already been requested this run.
    requested: ShotRequested,
}

impl CaptureProgress {
    /// Whether the screenshot has already been requested — the pipeline-wiring test's assertion
    /// hook (it drives the app past settle and checks this flipped, which is observable WITHOUT a
    /// GPU / a written PNG).
    #[must_use]
    pub const fn is_requested(&self) -> bool {
        self.requested.is_marked()
    }
}

/// `OnEnter(scene)`: reset the per-run capture counters so the settle window is measured from the
/// moment the capturing scene comes up. The consuming plugin registers this on its OWN `OnEnter(S)`
/// (the crate stays free of the concrete `States` type — see
/// [`ScreenshotCapturePlugin`](crate::ScreenshotCapturePlugin)).
pub fn reset_progress(mut progress: ResMut<CaptureProgress>) {
    *progress = CaptureProgress::default();
}

/// `Update`: after [`SettleFrames`] frames, spawn ONE primary-window
/// [`Screenshot`] with a [`save_to_disk`] observer (exactly once).
///
/// The GPU readback is async, so this does NOT exit here — the caller waits for the file
/// ([`poll_then_exit`] for a plain binary, or its own state-machine cascade). Param-only
/// (`bevy-traps.md` #7): [`Res`] reads + [`ResMut`] + [`Commands`], no `&mut World`.
///
/// The actual capture needs a real render device, so the PNG write is not headless-testable — but
/// the settle gate + the `ShotRequested` flip ARE (the pipeline-wiring test drives past settle and
/// asserts [`CaptureProgress::is_requested`]).
pub fn settle_then_capture(
    shot: Res<CapturePath>,
    settle: Res<SettleFrames>,
    mut progress: ResMut<CaptureProgress>,
    mut commands: Commands,
) {
    if progress.requested.is_marked() {
        return;
    }
    progress.frames.tick();
    if *progress.frames < **settle {
        return;
    }
    commands
        .spawn(Screenshot::primary_window())
        .observe(save_to_disk((**shot).clone()));
    progress.requested = ShotRequested::mark();
}

/// `Update`: once the screenshot has been requested, poll each frame until the PNG appears on disk,
/// then write `AppExit::Success`. A [`PollCap`] safety net prevents a failed write from hanging
/// the process.
///
/// This is the capture-then-exit path for a PLAIN binary with no game state machine (the content
/// editor). A game-state-machine consumer does NOT use this — it exits via its own
/// `RunningState::Quit` cascade instead (writing `AppExit` from a `ScreenshotCaptured` observer
/// does not reliably terminate winit on macOS — Bevy #23313). Uses a [`Local<u32>`] frame source,
/// not a `Res` counter, to avoid the macOS `Res<FrameCount>` hang (#24035). Param-only
/// (`bevy-traps.md` #7).
pub fn poll_then_exit(
    shot: Res<CapturePath>,
    progress: Res<CaptureProgress>,
    poll_cap: Res<PollCap>,
    mut poll_frames: Local<u32>,
    mut exit: MessageWriter<AppExit>,
) {
    if !progress.requested.is_marked() {
        return;
    }
    *poll_frames += 1;
    if shot.exists() {
        info!("gdtf_screenshot: screenshot written to {}", shot.display());
        exit.write(AppExit::Success);
        return;
    }
    if *poll_frames >= **poll_cap {
        warn!(
            "gdtf_screenshot: screenshot PNG not found after {} poll frames; giving up. Was the \
             capture path writable?",
            **poll_cap
        );
        exit.write(AppExit::Success);
    }
}

#[cfg(test)]
mod tests {
    use super::{CaptureProgress, FrameCount, ShotRequested};

    #[test]
    fn frame_count_ticks() {
        let mut frames = FrameCount::default();
        assert_eq!(*frames, 0);
        frames.tick();
        frames.tick();
        assert_eq!(*frames, 2);
    }

    #[test]
    fn shot_requested_starts_unmarked_and_marks() {
        assert!(!ShotRequested::default().is_marked());
        assert!(ShotRequested::mark().is_marked());
    }

    #[test]
    fn fresh_progress_is_not_requested() {
        assert!(!CaptureProgress::default().is_requested());
    }
}
