//! Parked replies that settle across frames and expire after a budget.

use core::time::Duration;
use std::{collections::VecDeque, time::Instant};

use bevy::prelude::*;
use cobalt_mcp_protocol::message::{QaError, QaResponse};

use super::CommandResponder;
use crate::command::QaCommand;

/// How long a deferred reply may wait before timing out.
#[derive(Deref, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct DeferredBudget(Duration);

impl DeferredBudget {
    /// Wrap a duration budget.
    #[must_use]
    pub const fn new(budget: Duration) -> Self {
        Self(budget)
    }
}

/// Default two-second budget for deferred replies.
pub const DEFERRED_BUDGET: DeferredBudget = DeferredBudget::new(Duration::from_secs(2));

/// How many deferred replies were delivered in one `answer_all`.
#[derive(Deref, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct DeliveredCount(usize);

impl DeliveredCount {
    /// Wrap a count.
    #[must_use]
    pub const fn new(count: usize) -> Self {
        Self(count)
    }
}

/// Outcome of answering the next parked reply.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum DeferredDelivery {
    /// A parked reply was answered.
    Delivered,
    /// Nothing was waiting.
    Empty,
}

#[derive(Deref, Debug, Clone, Copy, PartialEq, Eq)]
struct ParkedAt(Instant);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
enum ParkedAge {
    Live,
    Expired,
}

impl ParkedAt {
    fn now() -> Self {
        Self(Instant::now())
    }

    fn age_against(self, budget: DeferredBudget, now: Self) -> ParkedAge {
        if now.0.saturating_duration_since(self.0) >= *budget {
            return ParkedAge::Expired;
        }
        ParkedAge::Live
    }
}

struct ParkedReply<C: QaCommand> {
    responder: CommandResponder<C>,
    parked_at: ParkedAt,
    ticket:    C::Parked,
}

/// Bevy resource holding deferred responders for command `C`.
#[derive(Resource)]
pub struct DeferredReplies<C: QaCommand> {
    parked: VecDeque<ParkedReply<C>>,
    budget: DeferredBudget,
}

impl<C: QaCommand> Default for DeferredReplies<C> {
    fn default() -> Self {
        Self {
            parked: VecDeque::new(),
            budget: DEFERRED_BUDGET,
        }
    }
}

impl<C: QaCommand> DeferredReplies<C> {
    /// Empty parking with an explicit timeout budget.
    #[must_use]
    pub const fn with_budget(budget: DeferredBudget) -> Self {
        Self {
            parked: VecDeque::new(),
            budget,
        }
    }

    /// Park a responder, with the ticket this call waits on, to answer later.
    pub fn park(&mut self, responder: CommandResponder<C>, ticket: C::Parked) {
        self.parked.push_back(ParkedReply {
            responder,
            parked_at: ParkedAt::now(),
            ticket,
        });
    }

    /// Whether nothing is waiting.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.parked.is_empty()
    }

    /// Number of parked responders.
    #[must_use]
    pub fn len(&self) -> usize {
        self.parked.len()
    }

    /// Current timeout budget.
    #[must_use]
    pub const fn budget(&self) -> DeferredBudget {
        self.budget
    }

    /// Override the timeout budget (tests).
    pub const fn set_budget(&mut self, budget: DeferredBudget) {
        self.budget = budget;
    }

    /// Answer the oldest parked responder, or report empty.
    pub fn answer_next(&mut self, reply: &C::Reply) -> DeferredDelivery {
        match self.parked.pop_front() {
            Some(entry) => {
                entry.responder.answer(reply);
                DeferredDelivery::Delivered
            }
            None => DeferredDelivery::Empty,
        }
    }

    /// Answer every parked responder with the same reply.
    #[must_use = "the delivered count says whether anything was actually waiting"]
    pub fn answer_all(&mut self, reply: &C::Reply) -> DeliveredCount {
        let mut delivered = 0_usize;
        while let Some(entry) = self.parked.pop_front() {
            entry.responder.answer(reply);
            delivered += 1;
        }
        DeliveredCount::new(delivered)
    }

    /// Answer only the parked entries `resolve` hands a reply back for; keep the rest.
    #[must_use = "the delivered count says how many waiters the world just released"]
    pub fn answer_resolved<F>(&mut self, mut resolve: F) -> DeliveredCount
    where
        F: FnMut(&C::Parked) -> Option<C::Reply>,
    {
        let mut delivered = 0_usize;
        let mut kept = VecDeque::with_capacity(self.parked.len());
        while let Some(entry) = self.parked.pop_front() {
            match resolve(&entry.ticket) {
                Some(reply) => {
                    entry.responder.answer(&reply);
                    delivered += 1;
                }
                None => kept.push_back(entry),
            }
        }
        self.parked = kept;
        DeliveredCount::new(delivered)
    }

    fn sweep_expired(&mut self) {
        let budget = self.budget;
        let now = ParkedAt::now();
        let mut kept = VecDeque::with_capacity(self.parked.len());
        while let Some(entry) = self.parked.pop_front() {
            match entry.parked_at.age_against(budget, now) {
                ParkedAge::Expired => {
                    debug!(
                        command = C::NAME.as_str(),
                        "mcp: a deferred reply expired before it settled"
                    );
                    entry
                        .responder
                        .into_inner()
                        .reply(QaResponse::Error(QaError::Timeout));
                }
                ParkedAge::Live => kept.push_back(entry),
            }
        }
        self.parked = kept;
    }
}

/// Time out deferred replies that exceeded their budget.
pub fn sweep_deferred<C: QaCommand>(mut deferred: ResMut<DeferredReplies<C>>) {
    if deferred.is_empty() {
        return;
    }
    deferred.sweep_expired();
}
