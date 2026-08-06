//! The one settle → aim-check → spawn → verify → complete loop.

use bevy::{ecs::system::SystemParam, prelude::*};

use super::{
    aim::{CaptureAim, CaptureAimCheck},
    dir::{ShotDir, ShotSequence, next_capture_path},
    frames::{FrameTick, FramesLeft},
    outcome::CaptureOutcome,
    queue::{CaptureCompletions, CaptureQueue, CaptureStage, InFlightCapture},
    source::CaptureSource,
    spawn::{ensure_dir, spawn_capture},
    verify::{ShotFile, inspect_shot},
};
use crate::settle::{PollCap, SettleFrames};

#[derive(SystemParam)]
pub(super) struct CaptureTunables<'w> {
    settle: Res<'w, SettleFrames>,
    poll:   Res<'w, PollCap>,
    dir:    Res<'w, ShotDir>,
    source: Res<'w, CaptureSource>,
}

pub(super) fn drive_captures<P: Send + Sync + 'static>(
    mut queue: ResMut<CaptureQueue<P>>,
    mut completions: ResMut<CaptureCompletions<P>>,
    mut sequence: ResMut<ShotSequence>,
    tunables: CaptureTunables,
    aim: CaptureAimCheck,
    mut commands: Commands,
) {
    advance_in_flight(&mut queue, &mut completions, &tunables, &aim, &mut commands);
    claim_requests(&mut queue, &tunables, &mut sequence);
}

fn claim_requests<P: Send + Sync + 'static>(
    queue: &mut CaptureQueue<P>,
    tunables: &CaptureTunables<'_>,
    sequence: &mut ShotSequence,
) {
    for request in queue.take_queued() {
        let path = next_capture_path(&tunables.dir, request.stem.as_ref(), sequence);
        ensure_dir(&path);
        queue.keep(InFlightCapture {
            path,
            payload: request.payload,
            stage: CaptureStage::Settling(FramesLeft::new(**tunables.settle)),
        });
    }
}

fn advance_in_flight<P: Send + Sync + 'static>(
    queue: &mut CaptureQueue<P>,
    completions: &mut CaptureCompletions<P>,
    tunables: &CaptureTunables<'_>,
    aim: &CaptureAimCheck<'_, '_>,
    commands: &mut Commands<'_, '_>,
) {
    for mut capture in queue.take_in_flight() {
        match &mut capture.stage {
            CaptureStage::Settling(remaining) => {
                if let FrameTick::Live = remaining.tick() {
                    queue.keep(capture);
                    continue;
                }
                if let CaptureAim::Refused(detail) = aim.verify(&tunables.source) {
                    warn!(
                        detail = %detail.as_str(),
                        "gdtf_screenshot: refusing a capture of an image nothing renders into",
                    );
                    completions.push(CaptureOutcome::Refused(detail), capture.payload);
                    continue;
                }
                spawn_capture(&capture.path, &tunables.source, commands);
                capture.stage = CaptureStage::Capturing(FramesLeft::new(**tunables.poll));
                queue.keep(capture);
            }
            CaptureStage::Capturing(remaining) => match inspect_shot(&capture.path) {
                ShotFile::Ready => {
                    let landed = CaptureOutcome::Landed(capture.path.clone());
                    completions.push(landed, capture.payload);
                }
                ShotFile::NotReady => match remaining.tick() {
                    FrameTick::Expired => {
                        debug!(
                            path = %capture.path.display(),
                            "gdtf_screenshot: capture timed out before its PNG landed",
                        );
                        let timed_out = CaptureOutcome::TimedOut(capture.path.clone());
                        completions.push(timed_out, capture.payload);
                    }
                    FrameTick::Live => queue.keep(capture),
                },
            },
        }
    }
}
