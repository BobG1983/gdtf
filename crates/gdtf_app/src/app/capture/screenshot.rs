//! The `capture_when_ready` screenshot system — the render-device capture path of the
//! DEV-ONLY capture affordance. Split out of the sibling `plugin` module (GTW-583);
//! see its header for the full affordance rationale.

use bevy::{
    prelude::*,
    render::view::window::screenshot::{Screenshot, ScreenshotCaptured, save_to_disk},
};

use super::capture_config::{CaptureConfig, CaptureFrame, frame_path};
use crate::states::RunningState;

/// Captures the rendered battlescape frame(s) to disk on each scheduled
/// [`BattleRunning`](crate::states::BattleScapeState::BattleRunning) frame, then exits the app after the
/// last.
///
/// Runs in `Update`, gated `run_if(in_state(BattleScapeState::BattleRunning))`, so its
/// [`Local<u32>`] counter increments ONLY while the battle is on screen (it counts
/// frames since the battle rendered, not total app frames). Deliberately does NOT read
/// `Res<FrameCount>` (Bevy issue #24035 hang) — the `Local` is the frame source.
///
/// On each frame whose count matches a [`CaptureFrames`](super::capture_config::CaptureFrames) target it spawns a
/// [`Screenshot::primary_window`] entity with an observer that calls
/// [`save_to_disk`](bevy::render::view::window::screenshot::save_to_disk) to write that
/// frame's PNG (the exact path for the single-frame case, a `.fNN`-tagged path otherwise
/// — [`frame_path`]). The observer saves SYNCHRONOUSLY first (so the final PNG flushes),
/// then on the LAST target frame sets [`RunningState::Quit`] to ride the shared shutdown
/// cascade — Quit -> [`AppState::Teardown`](crate::states::AppState::Teardown) despawns
/// the `PrimaryWindow` (windowed/macOS native exit) and writes `AppExit` (headless
/// fallback). It does NOT write `AppExit` itself: an `AppExit` from an ordinary
/// observer does not reliably terminate winit on macOS (Bevy issue #23313, unfixed in
/// 0.18.1). GTW-311 fixed the teardown exit; GTW-316 routes this sibling capture path
/// through the same cascade so it exits cleanly instead of hanging.
///
/// Param-only (`bevy-traps.md` #7): [`Commands`] + [`Res`]`<`[`CaptureConfig`]`>` + a
/// [`Local<u32>`] — no `&mut World`. The actual capture needs a real render device, so
/// this is verified by RUNNING the app (the orchestrator), NOT in a headless test.
pub(super) fn capture_when_ready(
    mut commands: Commands,
    config: Res<CaptureConfig>,
    mut frames_in_battle: Local<u32>,
) {
    *frames_in_battle += 1;
    let current = CaptureFrame::new(*frames_in_battle);
    if !config.frames.contains(&current) {
        // Not a scheduled frame: wait.
        return;
    }
    let is_last = current == config.frames.last_frame();
    let path = if config.frames.is_single() {
        // Single-frame: write the exact GDTF_CAPTURE_PATH (GTW-297 behavior preserved).
        config.path.clone()
    } else {
        // Multi-frame: one PNG per frame, `.fNN`-tagged.
        frame_path(&config.path, current)
    };
    // GTW-590 C3: the trigger side of the loudness contract — one line per scheduled
    // frame naming its output path. The WRITE result is logged by bevy's `save_to_disk`
    // (`Screenshot saved to <path>` / `Cannot save screenshot ...`), so trigger + write
    // together leave no silent frame.
    info!(
        "dev-capture: capturing BattleRunning frame {} -> {}",
        *current,
        path.display(),
    );
    commands.spawn(Screenshot::primary_window()).observe(
        move |captured: On<ScreenshotCaptured>, mut next: ResMut<NextState<RunningState>>| {
            // Flush this frame's PNG to disk FIRST and synchronously (so the final image is
            // written before anything tears the app down). Then, only on the last scheduled
            // frame, hand off to the shared shutdown CASCADE by setting RunningState::Quit
            // (GTW-316): that drives the Quit scene -> AppState::Teardown, which despawns the
            // PrimaryWindow (windowed/macOS native exit, no #23313 hang) AND writes AppExit
            // (headless fallback). Writing AppExit directly from this observer does NOT
            // reliably terminate winit on macOS (Bevy issue #23313, not fixed in 0.18.1) —
            // GTW-311 fixed the teardown path but this sibling capture path bypassed it.
            save_to_disk(&path)(captured);
            if is_last {
                next.set(RunningState::Quit);
            }
        },
    );
}
