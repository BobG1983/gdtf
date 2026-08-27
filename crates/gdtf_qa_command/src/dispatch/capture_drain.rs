//! Hand each held reply's shot to the capture pipeline and report how it finished.

use bevy::prelude::*;
use gdtf_qa_protocol::command::ArtifactPath;
use gdtf_screenshot::{CaptureCompletions, CaptureOutcome, CaptureQueue, ShotStem};

use super::{CaptureHolds, CaptureTicket, RiderShot};

/// Queue the shots the holds have asked for, and answer the holds whose shots have finished.
pub fn drive_rider_captures(
    mut holds: ResMut<CaptureHolds>,
    mut captures: ResMut<CaptureQueue<CaptureTicket>>,
    mut completions: ResMut<CaptureCompletions<CaptureTicket>>,
) {
    for request in holds.drain_requests() {
        let stem = request.name().map(|name| ShotStem::new(name.as_str()));
        captures.push(stem, request.ticket());
    }
    if completions.is_empty() {
        return;
    }
    for completion in completions.drain() {
        let (outcome, ticket) = completion.into_parts();
        holds.complete(ticket, landed_or_lost(outcome));
    }
}

fn landed_or_lost(outcome: CaptureOutcome) -> RiderShot {
    match outcome {
        CaptureOutcome::Landed(path) => {
            RiderShot::Landed(ArtifactPath::new(path.to_string_lossy().into_owned()))
        }
        CaptureOutcome::TimedOut(path) => {
            debug!(
                path = %path.display(),
                "net_qa: a capture rider's screenshot timed out"
            );
            RiderShot::Lost
        }
    }
}
