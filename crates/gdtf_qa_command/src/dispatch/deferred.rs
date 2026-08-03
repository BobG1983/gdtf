use core::time::Duration;
use std::{collections::VecDeque, time::Instant};

use bevy::prelude::*;
use gdtf_qa_protocol::message::{QaError, QaResponse};

use super::CommandResponder;
use crate::command::QaCommand;

#[derive(Deref, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct DeferredBudget(Duration);

impl DeferredBudget {
        #[must_use]
    pub const fn new(budget: Duration) -> Self {
        Self(budget)
    }
}

pub const DEFERRED_BUDGET: DeferredBudget = DeferredBudget::new(Duration::from_secs(2));

#[derive(Deref, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct DeliveredCount(usize);

impl DeliveredCount {
        #[must_use]
    pub const fn new(count: usize) -> Self {
        Self(count)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum DeferredDelivery {
        Delivered,
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
}

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
        pub fn park(&mut self, responder: CommandResponder<C>) {
        self.parked.push_back(ParkedReply {
            responder,
            parked_at: ParkedAt::now(),
        });
    }

            #[must_use]
    pub fn is_empty(&self) -> bool {
        self.parked.is_empty()
    }

        #[must_use]
    pub fn len(&self) -> usize {
        self.parked.len()
    }

        #[must_use]
    pub const fn budget(&self) -> DeferredBudget {
        self.budget
    }

                    pub const fn set_budget(&mut self, budget: DeferredBudget) {
        self.budget = budget;
    }

            pub fn answer_next(&mut self, reply: &C::Reply) -> DeferredDelivery {
        match self.parked.pop_front() {
            Some(entry) => {
                entry.responder.answer(reply);
                DeferredDelivery::Delivered
            }
            None => DeferredDelivery::Empty,
        }
    }

        #[must_use = "the delivered count says whether anything was actually waiting"]
    pub fn answer_all(&mut self, reply: &C::Reply) -> DeliveredCount {
        let mut delivered = 0_usize;
        while let Some(entry) = self.parked.pop_front() {
            entry.responder.answer(reply);
            delivered += 1;
        }
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
                        "net_qa: a deferred reply expired before it settled"
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

pub fn sweep_deferred<C: QaCommand>(mut deferred: ResMut<DeferredReplies<C>>) {
    if deferred.is_empty() {
        return;
    }
    deferred.sweep_expired();
}
