//! The two `ScreenshotAfter` systems (GTW-749, the T15 child): the same-frame claim +
//! intent-inject, and the unconditional per-frame countdown + fire.

use bevy::prelude::*;
use gdtf_qa_protocol::envelope::{InjectReceipt, QaResponse, ScreenshotAfterResult};

use super::queue::AfterShotQueue;
use crate::dev::net_qa::{
    inject::receipt_for,
    pending::{PendingQueue, ScreenshotAfterPayload},
    present::QaCaptureTarget,
    resolve::{InjectActors, InjectQueues, RawInputSink},
    screenshot::{InFlightShots, QaShotDir, ShotPollBudget, ShotSequence},
};

/// Drain the routed [`ScreenshotAfterPayload`] queue: classify + push each request's
/// embedded intent through the EXACT SAME [`receipt_for`] classification the T4
/// `apply_injects` pump uses (never a parallel path) and answer accordingly —
/// [`Rejected`](ScreenshotAfterResult::Rejected) immediately (no capture, ever, on a
/// rejected intent), or queued into [`AfterShotQueue`] for [`tick_after_shots`] to fire
/// once its `frame_delay` elapses.
///
/// Registered in [`InputSystems::Gather`](gdtf_battle_input::InputSystems)
/// `.after(route_requests)` (so it sees the same frame's routed pushes),
/// `.after(tick_after_shots)` (so an entry it pushes is NEVER ticked the same frame it
/// is created — the T7 poll-before-claim discipline applied here too), and
/// `.before(ContextualActSystems::Drain)` + `.before(dispatch_act_intents)` (the T4
/// same-frame co-schedule guarantee: an intent this pushes is drained, and its
/// `*Requested` sim-consumed, the SAME frame). Gated on a live battle by the plugin — the
/// router already rejects an off-battle `ScreenshotAfter` `NoBattle` before it ever
/// reaches this queue (`bevy-traps.md` #1), exactly like a bare `Inject`.
pub(in crate::dev::net_qa) fn claim_screenshot_after(
    mut pending: ResMut<PendingQueue<ScreenshotAfterPayload>>,
    mut queues: InjectQueues,
    actors: InjectActors,
    mut raw_input: RawInputSink,
    mut after_shots: ResMut<AfterShotQueue>,
    mut commands: Commands,
) {
    for (payload, responder) in pending.drain_ready() {
        match receipt_for(
            payload.intent(),
            &mut queues,
            &actors,
            &mut raw_input,
            &mut commands,
        ) {
            InjectReceipt::Queued => {
                after_shots.push(payload.frame_delay(), payload.name().cloned(), responder);
            }
            InjectReceipt::Rejected(reason) => {
                responder.reply(QaResponse::ScreenshotAfter(
                    ScreenshotAfterResult::Rejected(reason),
                ));
            }
        }
    }
}

/// Tick every queued `ScreenshotAfter` capture, firing the real capture (via the T7
/// pump's [`spawn_capture`](super::super::screenshot::spawn_capture)) for any whose
/// countdown just elapsed.
///
/// Registered in [`InputSystems::Gather`](gdtf_battle_input::InputSystems)
/// `.after(route_requests)`, UNCONDITIONALLY (no battle gate — a capture already queued
/// must keep counting down and fire even if the battle ends before it does; only the
/// initial claim needs a live battle). Ordered `.before(claim_screenshot_after)` so a
/// freshly-claimed entry is ticked starting the NEXT frame, never the frame it is
/// created on.
pub(in crate::dev::net_qa) fn tick_after_shots(
    mut after_shots: ResMut<AfterShotQueue>,
    mut in_flight: ResMut<InFlightShots>,
    mut sequence: ResMut<ShotSequence>,
    budget: Res<ShotPollBudget>,
    dir: Res<QaShotDir>,
    capture_target: Option<Res<QaCaptureTarget>>,
    mut commands: Commands,
) {
    // GTW-764: same offscreen-vs-window capture-source choice as the T7 pump — pass the
    // present path's target through so a `ScreenshotAfter` capture is never a black window.
    after_shots.fire_due(
        &mut in_flight,
        *budget,
        &dir,
        &mut sequence,
        capture_target.as_deref(),
        &mut commands,
    );
}
