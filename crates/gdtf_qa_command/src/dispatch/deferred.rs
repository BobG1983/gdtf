//! [`DeferredReplies`] — the calls a handler parked to answer on a later frame — and the
//! deadline sweep that keeps one from hanging forever.

use core::time::Duration;
use std::{collections::VecDeque, time::Instant};

use bevy::prelude::*;
use gdtf_qa_protocol::message::{QaError, QaResponse};

use super::CommandResponder;
use crate::command::QaCommand;

/// How long a parked reply may wait before the sweep answers the client instead.
///
/// Private-inner newtype over [`Duration`] (no-bare-types). Distinct from the transport's
/// socket timeout by intent as well as by type: this one bounds the APP's own deferral, and
/// must expire first so the client learns the command never settled rather than watching
/// its socket time out.
#[derive(Deref, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct DeferredBudget(Duration);

impl DeferredBudget {
    /// Build a deferral budget from its duration.
    #[must_use]
    pub const fn new(budget: Duration) -> Self {
        Self(budget)
    }
}

/// The default deferral budget — deliberately, and provably, shorter than
/// [`DEFAULT_IO_TIMEOUT`](gdtf_net_qa_transport::DEFAULT_IO_TIMEOUT).
///
/// A deferred command that never settles must produce an ANSWER, not a dead socket. The
/// suite pins the inequality rather than trusting the two numbers to stay in step.
pub const DEFERRED_BUDGET: DeferredBudget = DeferredBudget::new(Duration::from_secs(2));

/// How many parked replies an answer delivered.
///
/// Private-inner newtype over `usize` (no-bare-types): a delivered count is not a length
/// and not an index.
#[derive(Deref, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct DeliveredCount(usize);

impl DeliveredCount {
    /// Build a delivered count.
    #[must_use]
    pub const fn new(count: usize) -> Self {
        Self(count)
    }
}

/// Whether a single-answer attempt found a parked call — a typed alternative to a bare
/// `bool`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum DeferredDelivery {
    /// One parked call was answered.
    Delivered,
    /// Nothing was parked.
    Empty,
}

/// The moment a reply was parked.
///
/// Private-inner newtype over [`Instant`] (no-bare-types): a park time is not any instant,
/// it is the one the budget is measured from.
#[derive(Deref, Debug, Clone, Copy, PartialEq, Eq)]
struct ParkedAt(Instant);

/// Whether a parked reply is still inside its budget — a typed alternative to a bare
/// `bool`, matching the transport's own `DeadlineTick`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
enum ParkedAge {
    /// Still within budget.
    Live,
    /// The budget is spent — answer the client with a timeout.
    Expired,
}

impl ParkedAt {
    /// The moment now.
    fn now() -> Self {
        Self(Instant::now())
    }

    /// Whether this park time has outlived `budget` as of `now`.
    fn age_against(self, budget: DeferredBudget, now: Self) -> ParkedAge {
        if now.0.saturating_duration_since(self.0) >= *budget {
            return ParkedAge::Expired;
        }
        ParkedAge::Live
    }
}

/// One parked reply and the moment it was parked.
struct ParkedReply<C: QaCommand> {
    /// The typed channel the answer will go out on.
    responder: CommandResponder<C>,
    /// When it was parked — the budget is measured from here.
    parked_at: ParkedAt,
}

/// The calls this command's handler parked to answer on a later frame.
///
/// The `Deferred` half of a command's timing: a handler that cannot answer yet parks its
/// [`CommandResponder`] here and returns, and a later frame's system answers it when the
/// world says so. Nothing here knows WHAT the command is waiting for — that is the
/// command's own business.
#[derive(Resource)]
pub struct DeferredReplies<C: QaCommand> {
    /// Parked replies, oldest first.
    parked: VecDeque<ParkedReply<C>>,
    /// How long a parked reply may wait.
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
    /// Park a responder to answer on a later frame.
    pub fn park(&mut self, responder: CommandResponder<C>) {
        self.parked.push_back(ParkedReply {
            responder,
            parked_at: ParkedAt::now(),
        });
    }

    /// Whether nothing is parked — the sweep's and a settle system's early-out, taking
    /// `&self` so reading it through `ResMut` does not dirty change detection.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.parked.is_empty()
    }

    /// How many replies are parked.
    #[must_use]
    pub fn len(&self) -> usize {
        self.parked.len()
    }

    /// The budget a newly parked reply gets.
    #[must_use]
    pub const fn budget(&self) -> DeferredBudget {
        self.budget
    }

    /// Set the budget parked replies are measured against.
    ///
    /// A command whose deferral is naturally longer or shorter than the default says so
    /// here; a test drives it to zero to make expiry deterministic instead of sleeping.
    pub const fn set_budget(&mut self, budget: DeferredBudget) {
        self.budget = budget;
    }

    /// Answer the OLDEST parked call, leaving the rest parked with their own deadlines
    /// intact.
    pub fn answer_next(&mut self, reply: &C::Reply) -> DeferredDelivery {
        match self.parked.pop_front() {
            Some(entry) => {
                entry.responder.answer(reply);
                DeferredDelivery::Delivered
            }
            None => DeferredDelivery::Empty,
        }
    }

    /// Answer every parked call with the same reply.
    #[must_use = "the delivered count says whether anything was actually waiting"]
    pub fn answer_all(&mut self, reply: &C::Reply) -> DeliveredCount {
        let mut delivered = 0_usize;
        while let Some(entry) = self.parked.pop_front() {
            entry.responder.answer(reply);
            delivered += 1;
        }
        DeliveredCount::new(delivered)
    }

    /// Answer anything past its budget with a deadline reply and drop it, keeping the rest.
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

/// The deferral deadline pump for ONE command, registered by
/// [`register_command`](super::register_command).
///
/// It answers [`Timeout`](QaError::Timeout) on a parked reply that never settled. That
/// answer has to arrive while the client is still listening, which is why
/// [`DEFERRED_BUDGET`] sits strictly inside
/// [`DEFAULT_IO_TIMEOUT`](gdtf_net_qa_transport::DEFAULT_IO_TIMEOUT): expiring first is the
/// difference between "the app told me it never settled" and "my socket died".
pub fn sweep_deferred<C: QaCommand>(mut deferred: ResMut<DeferredReplies<C>>) {
    // Read emptiness through the immutable accessor so an idle frame never dirties the
    // resource's change-detection flag.
    if deferred.is_empty() {
        return;
    }
    deferred.sweep_expired();
}
