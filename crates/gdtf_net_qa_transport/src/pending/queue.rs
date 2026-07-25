//! The typed pending-request FIFO and its entries (GTW-736).

use std::collections::VecDeque;

use bevy::prelude::*;
use gdtf_qa_protocol::envelope::{QaError, QaResponse};

use super::deadline::{DEADLINE_BUDGET, DeadlineTick, FrameDeadline};
use crate::channel::Responder;

/// One queued request awaiting a consumer: the payload, the reply channel, and the
/// countdown after which the sweep answers [`Timeout`](QaError::Timeout).
struct Pending<P> {
    /// The kind-specific payload a consumer reads.
    payload:   P,
    /// The reply channel back to the client.
    responder: Responder,
    /// Frames remaining before the sweep times it out.
    deadline:  FrameDeadline,
}

impl<P> Pending<P> {
    /// Queue a payload with the standard `DEADLINE_BUDGET`.
    const fn new(payload: P, responder: Responder) -> Self {
        Self {
            payload,
            responder,
            deadline: DEADLINE_BUDGET,
        }
    }
}

/// A typed FIFO of pending requests of one kind — a Bevy [`Resource`] per payload type,
/// so each of the host's consumers reads only its own queue.
#[derive(Resource)]
pub struct PendingQueue<P: Send + Sync + 'static>(VecDeque<Pending<P>>);

impl<P: Send + Sync + 'static> Default for PendingQueue<P> {
    fn default() -> Self {
        Self(VecDeque::new())
    }
}

impl<P: Send + Sync + 'static> PendingQueue<P> {
    /// Whether the queue holds no pending requests (the sweep's early-out — reading it
    /// through `ResMut` does not dirty change detection).
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    /// Enqueue a payload with the standard deadline and its reply channel.
    pub fn push_new(&mut self, payload: P, responder: Responder) {
        self.0.push_back(Pending::new(payload, responder));
    }

    /// Drain every currently-queued request, yielding each payload with its
    /// [`Responder`] — the same-frame consumer's read. The deadline
    /// [`sweep_pending`](super::sweep_pending) only reaps entries left UNCLAIMED; a
    /// consumer that answers a request the frame it is routed pulls them here first
    /// (running `.after` the router, so it sees this frame's pushes), so a consumed entry
    /// never reaches the sweep.
    #[must_use]
    pub fn drain_ready(&mut self) -> Vec<(P, Responder)> {
        self.0
            .drain(..)
            .map(|entry| (entry.payload, entry.responder))
            .collect()
    }

    /// Tick every entry once; answer [`Timeout`](QaError::Timeout) on any that expired
    /// unclaimed and drop it, keeping the rest.
    pub(super) fn sweep_expired(&mut self)
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
