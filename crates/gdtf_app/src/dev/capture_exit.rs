//! The ONE game-side capture EXIT: poll the captured PNG onto disk, then quit via the
//! shared [`RunningState::Quit`] shutdown cascade (GTW-577 C5).
//!
//! `gdtf_screenshot`'s own [`poll_then_exit`](gdtf_screenshot::poll_then_exit) writes
//! `AppExit` directly — correct for a PLAIN binary (the content editor) but NOT for the
//! game: an `AppExit` written outside the Teardown scene does not reliably terminate winit
//! on macOS (Bevy #23313). This gdtf_app-local sibling therefore NEVER writes `AppExit`; it
//! sets [`RunningState::Quit`], and the Quit → `AppState::Teardown` cascade exits correctly
//! for both windowed and headless runs — the same cascade every other "quit now" site rides.
//!
//! It serves ONLY the two capture scenes that already exit after their shot today (the
//! gang-editor and procgen-visualizer hooks, which chain it after the crate's
//! [`settle_then_capture`](gdtf_screenshot::settle_then_capture)); the loading-screen hook
//! deliberately has NO exit (its documented contract is capture-and-CONTINUE into the
//! battle).

use bevy::prelude::*;
use gdtf_screenshot::{CapturePath, CaptureProgress, PollCap};

use crate::states::RunningState;

crate::support_item! {
    /// `Update` (registered per capture scene, chained after
    /// [`settle_then_capture`](gdtf_screenshot::settle_then_capture)): once the screenshot has
    /// been REQUESTED, poll each frame until the PNG appears on disk, then set
    /// [`RunningState::Quit`] — the shared shutdown cascade. A [`PollCap`] safety net quits
    /// anyway (with a `warn!`) if the PNG never lands, so a failed write can never hang an
    /// unattended QA run.
    ///
    /// NEVER writes `AppExit` (the macOS winit hang, Bevy #23313) — the exit is the
    /// [`NextState`] write, and Teardown owns the eventual `AppExit`. Uses a [`Local<u32>`]
    /// frame source, not a `Res` counter (the macOS `Res<FrameCount>` hang, #24035); each
    /// registering scene gets its own instance, so the poll counters never interleave.
    /// Param-only (`bevy-traps.md` #7).
    fn poll_then_quit(
        shot: Res<CapturePath>,
        progress: Res<CaptureProgress>,
        poll_cap: Res<PollCap>,
        mut poll_frames: Local<u32>,
        mut next: ResMut<NextState<RunningState>>,
    ) {
        if !progress.is_requested() {
            return;
        }
        *poll_frames += 1;
        if shot.exists() {
            info!("capture: screenshot written to {}", shot.display());
            next.set(RunningState::Quit);
            return;
        }
        if *poll_frames >= **poll_cap {
            warn!(
                "capture: screenshot PNG not found after {} poll frames; quitting anyway. Was \
                 the capture path writable?",
                **poll_cap
            );
            next.set(RunningState::Quit);
        }
    }
}
