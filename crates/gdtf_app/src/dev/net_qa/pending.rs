//! The typed pending queues + the frame-deadline sweep (GTW-736).
//!
//! Requests the router cannot answer synchronously (an [`Inject`], a snapshot, an event
//! drain, a screenshot, a battle start) land in a per-kind [`PendingQueue`] where a
//! LATER child (T4-T7 / T9) consumes them. Every entry carries a [`FrameDeadline`]
//! countdown; the [`sweep_pending`] pump decrements it each frame and answers the client
//! with a [`Timeout`](gdtf_qa_protocol::envelope::QaError::Timeout) when it expires
//! unclaimed — so a request never leaves the client hanging, and (until the consumers
//! land) every enqueued request times out cleanly.

use std::collections::VecDeque;

use bevy::{ecs::system::SystemParam, prelude::*};
use gdtf_qa_protocol::{
    envelope::{QaError, QaResponse},
    ids::{EventCap, SeedNet, ShotName, SituationRef},
    intent::NetIntent,
};

use super::channel::Responder;

/// The grace, in frames, a pending request is given before the sweep times it out.
///
/// A real consumer (T4-T7) picks its queue entry up within a frame or two; this budget
/// only bounds how long an UNCLAIMED request waits before the client gets a prompt
/// [`Timeout`](gdtf_qa_protocol::envelope::QaError::Timeout) instead of a hang.
const DEADLINE_BUDGET: FrameDeadline = FrameDeadline::new(4);

/// The pending payload for a [`GetBattleState`](gdtf_qa_protocol::envelope::QaRequest::GetBattleState)
/// snapshot — consumed by the T5 snapshot child. Carries no data (the request has no
/// arguments); it exists so the snapshot queue is a distinct type.
#[derive(Debug)]
pub(super) struct SnapshotPayload;

/// The pending payload for an [`Inject`](gdtf_qa_protocol::envelope::QaRequest::Inject) —
/// consumed by the T4 intent-injection child.
pub(super) struct InjectPayload(NetIntent);

/// The pending payload for a [`GetOutput`](gdtf_qa_protocol::envelope::QaRequest::GetOutput)
/// event drain — consumed by the T6 outbox child. The optional cap the client asked for.
pub(super) struct OutputPayload(Option<EventCap>);

/// The pending payload for a [`TakeScreenshot`](gdtf_qa_protocol::envelope::QaRequest::TakeScreenshot)
/// — consumed by the T7 screenshot child. The optional file stem the client asked for.
pub(super) struct ScreenshotPayload(Option<ShotName>);

/// The pending payload for a [`StartBattle`](gdtf_qa_protocol::envelope::QaRequest::StartBattle)
/// — consumed by the T9 navigation child.
pub(super) struct StartBattlePayload {
    /// The situation to start.
    situation: SituationRef,
    /// The seed to pin, or `None` for a server-chosen seed.
    seed:      Option<SeedNet>,
}

impl InjectPayload {
    /// Wrap the injected intent.
    pub(super) const fn new(intent: NetIntent) -> Self {
        Self(intent)
    }

    /// The wrapped intent — the T4 [`apply_injects`](super::inject::apply_injects)
    /// consumer's read ([`NetIntent`] is `Copy`, so this borrows without consuming).
    pub(super) const fn intent(&self) -> NetIntent {
        self.0
    }
}

impl OutputPayload {
    /// Wrap the optional drain cap.
    pub(super) const fn new(cap: Option<EventCap>) -> Self {
        Self(cap)
    }
}

impl ScreenshotPayload {
    /// Wrap the optional screenshot stem.
    pub(super) const fn new(name: Option<ShotName>) -> Self {
        Self(name)
    }

    /// The wrapped stem — the T7 [`drive_screenshots`](super::screenshot::drive_screenshots)
    /// consumer's read (borrows without consuming; the pump confines it into a path).
    pub(super) const fn name(&self) -> Option<&ShotName> {
        self.0.as_ref()
    }
}

impl StartBattlePayload {
    /// Pair the situation with its optional seed.
    pub(super) const fn new(situation: SituationRef, seed: Option<SeedNet>) -> Self {
        Self { situation, seed }
    }
}

// Manual `Debug` impls (NOT derived) so the sweep's timeout diagnostic — the sole T3
// reader of these forward-declared payloads — genuinely reads each field: a derived
// `Debug` is ignored by dead-code analysis, an explicit `self.field` read is not.
impl core::fmt::Debug for InjectPayload {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_tuple("InjectPayload").field(&self.0).finish()
    }
}

impl core::fmt::Debug for OutputPayload {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_tuple("OutputPayload").field(&self.0).finish()
    }
}

impl core::fmt::Debug for ScreenshotPayload {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_tuple("ScreenshotPayload").field(&self.0).finish()
    }
}

impl core::fmt::Debug for StartBattlePayload {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("StartBattlePayload")
            .field("situation", &self.situation)
            .field("seed", &self.seed)
            .finish()
    }
}

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
pub(super) struct PendingQueue<P: Send + Sync + 'static>(VecDeque<Pending<P>>);

impl<P: Send + Sync + 'static> Default for PendingQueue<P> {
    fn default() -> Self {
        Self(VecDeque::new())
    }
}

impl<P: Send + Sync + 'static> PendingQueue<P> {
    /// Whether the queue holds no pending requests (the sweep's early-out — reading it
    /// through `ResMut` does not dirty change detection).
    pub(super) fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    /// Enqueue a payload with the standard deadline and its reply channel.
    pub(super) fn push_new(&mut self, payload: P, responder: Responder) {
        self.0.push_back(Pending::new(payload, responder));
    }

    /// Drain every currently-queued request, yielding each payload with its
    /// [`Responder`] — the same-frame consumer's read (the T4 injection pump). The
    /// deadline [`sweep_pending`] only reaps entries left UNCLAIMED; a consumer that
    /// answers a request the frame it is routed pulls them here first (running
    /// `.after` the router, so it sees this frame's pushes), so a consumed entry never
    /// reaches the sweep.
    pub(super) fn drain_ready(&mut self) -> Vec<(P, Responder)> {
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
pub(super) struct PendingQueues<'w> {
    /// Injected intents (T4).
    pub(super) inject:       ResMut<'w, PendingQueue<InjectPayload>>,
    /// Battle-state snapshots (T5).
    pub(super) snapshot:     ResMut<'w, PendingQueue<SnapshotPayload>>,
    /// Event drains (T6).
    pub(super) output:       ResMut<'w, PendingQueue<OutputPayload>>,
    /// Screenshots (T7).
    pub(super) screenshot:   ResMut<'w, PendingQueue<ScreenshotPayload>>,
    /// Battle starts (T9).
    pub(super) start_battle: ResMut<'w, PendingQueue<StartBattlePayload>>,
}

/// The deadline pump for ONE pending-queue kind — ticks every entry and times out any
/// that expired unclaimed. Registered once per payload type by the plugin.
pub(super) fn sweep_pending<P: Send + Sync + core::fmt::Debug + 'static>(
    mut queue: ResMut<PendingQueue<P>>,
) {
    // Read emptiness through the immutable accessor so an idle frame never dirties the
    // resource's change-detection flag.
    if queue.is_empty() {
        return;
    }
    queue.sweep_expired();
}
