//! Systems that wait, capture, then exit once the PNG is on disk.

use bevy::{
    prelude::*,
    render::view::window::screenshot::{Screenshot, save_to_disk},
};

use crate::{
    path::CapturePath,
    settle::{PollCap, SettleFrames},
};

/// Frames elapsed since settle began.
#[derive(Clone, Copy, Default, Debug, Deref)]
pub struct FrameCount(u32);

impl FrameCount {
    /// Advance by one frame.
    pub const fn tick(&mut self) {
        self.0 += 1;
    }
}

/// Whether the screenshot request has already been issued.
#[derive(Clone, Copy, Default, Debug, Deref)]
pub struct ShotRequested(bool);

impl ShotRequested {
    /// Mark as requested.
    #[must_use]
    pub const fn mark() -> Self {
        Self(true)
    }

    /// True when a shot was requested.
    #[must_use]
    pub const fn is_marked(self) -> bool {
        self.0
    }
}

/// Runtime progress for the automated capture path.
#[derive(Resource, Default, Debug)]
pub struct CaptureProgress {
    frames:    FrameCount,
    requested: ShotRequested,
}

impl CaptureProgress {
    /// True when the capture has been requested.
    #[must_use]
    pub const fn is_requested(&self) -> bool {
        self.requested.is_marked()
    }
}

/// Reset progress (e.g. before a new capture run).
pub fn reset_progress(mut progress: ResMut<CaptureProgress>) {
    *progress = CaptureProgress::default();
}

/// After [`SettleFrames`] have passed, spawn a primary-window screenshot.
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

/// Once a shot was requested, exit when the file exists or [`PollCap`] is hit.
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
