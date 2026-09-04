//! Frame-budgeted queue of unclaimed QA requests.

use std::collections::VecDeque;

use bevy::prelude::*;
use cobalt_mcp_protocol::message::{QaError, QaResponse};

use super::deadline::{DEADLINE_BUDGET, DeadlineTick, FrameDeadline};
use crate::channel::Responder;

struct Pending<P> {
    payload:   P,
    responder: Responder,
    deadline:  FrameDeadline,
}

impl<P> Pending<P> {
    const fn new(payload: P, responder: Responder) -> Self {
        Self {
            payload,
            responder,
            deadline: DEADLINE_BUDGET,
        }
    }
}

/// Pending requests waiting to be claimed by a system.
#[derive(Resource)]
pub struct PendingQueue<P: Send + Sync + 'static>(VecDeque<Pending<P>>);

impl<P: Send + Sync + 'static> Default for PendingQueue<P> {
    fn default() -> Self {
        Self(VecDeque::new())
    }
}

impl<P: Send + Sync + 'static> PendingQueue<P> {
    /// Whether the queue is empty.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    /// Enqueue a new request with a fresh deadline.
    pub fn push_new(&mut self, payload: P, responder: Responder) {
        self.0.push_back(Pending::new(payload, responder));
    }

    /// Drain every entry as ready work (does not check deadlines).
    #[must_use]
    pub fn drain_ready(&mut self) -> Vec<(P, Responder)> {
        self.0
            .drain(..)
            .map(|entry| (entry.payload, entry.responder))
            .collect()
    }

    pub(super) fn sweep_expired(&mut self)
    where
        P: core::fmt::Debug,
    {
        let mut kept = VecDeque::with_capacity(self.0.len());
        while let Some(mut entry) = self.0.pop_front() {
            match entry.deadline.tick() {
                DeadlineTick::Expired => {
                    debug!(request = ?entry.payload, "mcp: pending request timed out unclaimed");
                    entry.responder.reply(QaResponse::Error(QaError::Timeout));
                }
                DeadlineTick::Live => kept.push_back(entry),
            }
        }
        self.0 = kept;
    }
}
