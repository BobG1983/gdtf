//! The typed pending queue, its per-entry frame-deadline, the sweep pump, and the
//! [`PendingQueues`] router bundle (GTW-736; split out of the payloads per module-layout).
//!
//! Every [`PendingQueue`] entry carries a [`FrameDeadline`] countdown; the
//! [`sweep_pending`] pump decrements it each frame and answers the client with a
//! [`Timeout`](gdtf_qa_protocol::envelope::QaError::Timeout) when it expires unclaimed — so
//! a request never leaves the client hanging. The payload types the queues hold live in the
//! sibling [`payloads`](super::payloads) module.

use std::collections::VecDeque;

use bevy::{ecs::system::SystemParam, prelude::*};
use gdtf_qa_protocol::envelope::{QaError, QaResponse};

use super::payloads::{
    ActivateMenuPayload, InjectPayload, OutputPayload, ScreenshotAfterPayload, ScreenshotPayload,
    SnapshotPayload, StartBattlePayload, StepperControlPayload,
};
use crate::dev::net_qa::channel::Responder;

/// The grace, in frames, a pending request is given before the sweep times it out.
///
/// A real consumer (T4-T7) picks its queue entry up within a frame or two; this budget
/// only bounds how long an UNCLAIMED request waits before the client gets a prompt
/// [`Timeout`](gdtf_qa_protocol::envelope::QaError::Timeout) instead of a hang.
const DEADLINE_BUDGET: FrameDeadline = FrameDeadline::new(4);

/// A frames-remaining countdown after which an unclaimed pending request times out.
///
/// Private-inner newtype over `u32` (no-bare-types): the sweep [`tick`](Self::tick)s it
/// once per frame and answers [`Timeout`](gdtf_qa_protocol::envelope::QaError::Timeout)
/// on the tick it reaches zero.
#[derive(Debug, Clone, Copy)]
struct FrameDeadline(u32);

/// The outcome of ticking a [`FrameDeadline`] — a typed alternative to a bare `bool`.
enum DeadlineTick {
    /// Still within budget this frame.
    Live,
    /// The budget is spent — answer the client with a timeout.
    Expired,
}

impl FrameDeadline {
    /// Build a countdown of `frames` remaining.
    const fn new(frames: u32) -> Self {
        Self(frames)
    }

    /// Spend one frame of the budget, reporting whether it is now exhausted.
    const fn tick(&mut self) -> DeadlineTick {
        if self.0 == 0 {
            return DeadlineTick::Expired;
        }
        self.0 -= 1;
        DeadlineTick::Live
    }
}

/// One queued request awaiting a consumer: the payload, the reply channel, and the
/// countdown after which the sweep answers [`Timeout`](gdtf_qa_protocol::envelope::QaError::Timeout).
struct Pending<P> {
    /// The kind-specific payload a consumer reads.
    payload:   P,
    /// The reply channel back to the client.
    responder: Responder,
    /// Frames remaining before the sweep times it out.
    deadline:  FrameDeadline,
}

impl<P> Pending<P> {
    /// Queue a payload with the standard [`DEADLINE_BUDGET`].
    const fn new(payload: P, responder: Responder) -> Self {
        Self {
            payload,
            responder,
            deadline: DEADLINE_BUDGET,
        }
    }
}

/// A typed FIFO of pending requests of one kind — a Bevy [`Resource`] per payload type,
/// so each consumer (T4-T7 / T9) reads only its own queue.
#[derive(Resource)]
pub(in crate::dev::net_qa) struct PendingQueue<P: Send + Sync + 'static>(VecDeque<Pending<P>>);

impl<P: Send + Sync + 'static> Default for PendingQueue<P> {
    fn default() -> Self {
        Self(VecDeque::new())
    }
}

impl<P: Send + Sync + 'static> PendingQueue<P> {
    /// Whether the queue holds no pending requests (the sweep's early-out — reading it
    /// through `ResMut` does not dirty change detection).
    pub(in crate::dev::net_qa) fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    /// Enqueue a payload with the standard deadline and its reply channel.
    pub(in crate::dev::net_qa) fn push_new(&mut self, payload: P, responder: Responder) {
        self.0.push_back(Pending::new(payload, responder));
    }

    /// Drain every currently-queued request, yielding each payload with its
    /// [`Responder`] — the same-frame consumer's read (the T4 injection pump). The
    /// deadline [`sweep_pending`] only reaps entries left UNCLAIMED; a consumer that
    /// answers a request the frame it is routed pulls them here first (running
    /// `.after` the router, so it sees this frame's pushes), so a consumed entry never
    /// reaches the sweep.
    pub(in crate::dev::net_qa) fn drain_ready(&mut self) -> Vec<(P, Responder)> {
        self.0
            .drain(..)
            .map(|entry| (entry.payload, entry.responder))
            .collect()
    }

    /// Tick every entry once; answer [`Timeout`](gdtf_qa_protocol::envelope::QaError::Timeout)
    /// on any that expired unclaimed and drop it, keeping the rest.
    fn sweep_expired(&mut self)
    where
        P: core::fmt::Debug,
    {
        let mut kept = VecDeque::with_capacity(self.0.len());
        while let Some(mut entry) = self.0.pop_front() {
            match entry.deadline.tick() {
                DeadlineTick::Expired => {
                    debug!(request = ?entry.payload, "net_qa: pending request timed out unclaimed");
                    entry.responder.reply(QaResponse::Error(QaError::Timeout));
                }
                DeadlineTick::Live => kept.push_back(entry),
            }
        }
        self.0 = kept;
    }
}

/// The bundle of every typed pending queue the router enqueues into — one
/// [`SystemParam`] so the router stays under the argument-count ceiling.
#[derive(SystemParam)]
pub(in crate::dev::net_qa) struct PendingQueues<'w> {
    /// Injected intents (T4).
    pub(in crate::dev::net_qa) inject:           ResMut<'w, PendingQueue<InjectPayload>>,
    /// Battle-state snapshots (T5).
    pub(in crate::dev::net_qa) snapshot:         ResMut<'w, PendingQueue<SnapshotPayload>>,
    /// Event drains (T6).
    pub(in crate::dev::net_qa) output:           ResMut<'w, PendingQueue<OutputPayload>>,
    /// Screenshots (T7).
    pub(in crate::dev::net_qa) screenshot:       ResMut<'w, PendingQueue<ScreenshotPayload>>,
    /// Screenshot-after requests (T15) — battle-dependent (the embedded intent needs a
    /// live battle), gated at route time exactly like a bare `Inject`.
    pub(in crate::dev::net_qa) screenshot_after: ResMut<'w, PendingQueue<ScreenshotAfterPayload>>,
    /// Battle starts (T9).
    pub(in crate::dev::net_qa) start_battle:     ResMut<'w, PendingQueue<StartBattlePayload>>,
    /// DEV procgen stepper-drive commands (GTW-766) — gated at route time on a live
    /// `StagedProcgen` drive, not on a battle.
    pub(in crate::dev::net_qa) stepper_control:  ResMut<'w, PendingQueue<StepperControlPayload>>,
    /// Menu-item activations (GTW-787) — always serviceable at route time (the consumer
    /// validates the token against the live menu and answers a stale one).
    pub(in crate::dev::net_qa) activate_menu:    ResMut<'w, PendingQueue<ActivateMenuPayload>>,
}

/// The deadline pump for ONE pending-queue kind — ticks every entry and times out any
/// that expired unclaimed. Registered once per payload type by the plugin.
pub(in crate::dev::net_qa) fn sweep_pending<P: Send + Sync + core::fmt::Debug + 'static>(
    mut queue: ResMut<PendingQueue<P>>,
) {
    // Read emptiness through the immutable accessor so an idle frame never dirties the
    // resource's change-detection flag.
    if queue.is_empty() {
        return;
    }
    queue.sweep_expired();
}
