//! The transport-half registration: the typed pending queues, the cross-frame pump state,
//! the deadline sweeps, and the always-on router (GTW-736).
//!
//! The queue type and the sweep system are the shared transport's
//! ([`gdtf_net_qa_transport`], GTW-803); this file is where THIS host instantiates them —
//! one queue and one sweep per battle payload type, in the ordering the router needs.

use bevy::prelude::*;
use gdtf_battle_input::InputSystems;
use gdtf_net_qa_transport::{PendingQueue, sweep_pending};

use crate::dev::net_qa::{
    events::QaOutputCursor,
    pending::{
        ActivateMenuPayload, FocusControlPayload, InjectPayload, OutputPayload,
        ScreenshotAfterPayload, ScreenshotPayload, SnapshotPayload, StartBattlePayload,
        StepperControlPayload,
    },
    router::route_requests,
    screenshot::{InFlightShots, QaShotDir, ShotPollBudget, ShotSequence},
    screenshot_after::AfterShotQueue,
};

/// Init every typed pending queue plus the pumps' cross-frame state, and register the
/// deadline sweeps + the always-on router in the [`InputSystems::Gather`] band (sweeps
/// chained before the router, so a freshly-enqueued request gets its full deadline budget).
pub(super) fn register_transport(app: &mut App) {
    app.init_resource::<PendingQueue<InjectPayload>>()
        .init_resource::<PendingQueue<SnapshotPayload>>()
        .init_resource::<PendingQueue<OutputPayload>>()
        .init_resource::<PendingQueue<ScreenshotPayload>>()
        .init_resource::<PendingQueue<ScreenshotAfterPayload>>()
        .init_resource::<PendingQueue<StartBattlePayload>>()
        // GTW-766 — the DEV stepper-drive command queue.
        .init_resource::<PendingQueue<StepperControlPayload>>()
        // GTW-787 — the menu-item activation queue.
        .init_resource::<PendingQueue<ActivateMenuPayload>>()
        // GTW-802 — the focus-drive command queue.
        .init_resource::<PendingQueue<FocusControlPayload>>()
        // GTW-740 — the T7 screenshot pump's own state: the captures in-flight across
        // frames, the poll budget, the confinement directory, and the monotonic sequence
        // that makes every capture's output path unique.
        .init_resource::<InFlightShots>()
        .init_resource::<ShotPollBudget>()
        .init_resource::<QaShotDir>()
        .init_resource::<ShotSequence>()
        // GTW-749 — the T15 screenshot-after child's own state: the accepted requests
        // counting down to their fire frame.
        .init_resource::<AfterShotQueue>()
        // GTW-739 — the T6 outbox's own read position over the act log. Process-lifetime
        // (it self-heals across battles), so it is init'd here beside the other pump state
        // rather than at a battle boundary.
        .init_resource::<QaOutputCursor>();
    // NOTE: neither the `ScreenshotPayload` nor the `ScreenshotAfterPayload` queue has a
    // deadline sweep — the T7 pump (`drive_screenshots`) claims every screenshot the frame
    // it is routed and owns its own multi-frame poll timeout, and the T15
    // `claim_screenshot_after` claims every screenshot-after the frame it is routed too
    // (answering `Rejected` immediately or handing it to `AfterShotQueue`'s OWN countdown)
    // — a generic sweep would only ever answer the wrong reply type for either. Every
    // OTHER queue keeps its sweep.
    app.add_systems(
        Update,
        (
            sweep_pending::<InjectPayload>,
            sweep_pending::<SnapshotPayload>,
            sweep_pending::<OutputPayload>,
            sweep_pending::<StartBattlePayload>,
            sweep_pending::<StepperControlPayload>,
            sweep_pending::<ActivateMenuPayload>,
            sweep_pending::<FocusControlPayload>,
            route_requests,
        )
            .chain()
            .in_set(InputSystems::Gather),
    );
}
