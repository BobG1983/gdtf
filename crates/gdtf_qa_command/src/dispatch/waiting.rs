//! Calls held while their command's admission is re-tested each frame.

use core::time::Duration;
use std::time::Instant;

use bevy::prelude::*;
use gdtf_net_qa_transport::Responder;
use gdtf_qa_protocol::command::{AwaitBudget, CommandArgsRon, CommandName, RunOptions};

/// When a held call started waiting.
#[derive(Deref, Debug, Clone, Copy, PartialEq, Eq)]
pub struct WaitingSince(Instant);

impl WaitingSince {
    /// Start the clock now.
    #[must_use]
    pub fn now() -> Self {
        Self(Instant::now())
    }

    fn age_against(self, budget: AwaitBudget, now: Self) -> WaitingAge {
        if now.0.saturating_duration_since(self.0) >= Duration::from_secs(*budget) {
            return WaitingAge::Expired;
        }
        WaitingAge::Live
    }
}

/// Whether a held call still has budget left.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum WaitingAge {
    /// Budget left, so admission is tested again.
    Live,
    /// Budget spent, so the last refusal is the answer.
    Expired,
}

/// One call whose admission is re-tested until it admits or the budget runs out.
pub struct WaitingCall {
    name:      CommandName,
    arguments: CommandArgsRon,
    options:   RunOptions,
    responder: Responder,
    since:     WaitingSince,
    budget:    AwaitBudget,
}

impl WaitingCall {
    /// Start holding a refused call for the seconds the caller asked for.
    #[must_use]
    pub fn new(
        name: CommandName,
        arguments: CommandArgsRon,
        options: RunOptions,
        responder: Responder,
        budget: AwaitBudget,
    ) -> Self {
        Self {
            name,
            arguments,
            options,
            responder,
            since: WaitingSince::now(),
            budget,
        }
    }

    /// Command name this call targets.
    #[must_use]
    pub const fn name(&self) -> &CommandName {
        &self.name
    }

    /// Whether this call still has budget left at `now`.
    #[must_use]
    pub fn age_at(&self, now: WaitingSince) -> WaitingAge {
        self.since.age_against(self.budget, now)
    }

    /// Split into the parts an admission needs.
    #[must_use]
    pub fn into_parts(self) -> (CommandName, CommandArgsRon, RunOptions, Responder) {
        (self.name, self.arguments, self.options, self.responder)
    }

    /// Take the responder to answer this call directly.
    #[must_use]
    pub fn into_responder(self) -> Responder {
        self.responder
    }
}

/// Bevy resource holding calls whose admission is re-tested each frame.
#[derive(Resource, Default)]
pub struct WaitingCalls(Vec<WaitingCall>);

impl WaitingCalls {
    /// Hold a call for a later re-test.
    pub fn hold(&mut self, call: WaitingCall) {
        self.0.push(call);
    }

    /// Whether nothing is waiting.
    #[must_use]
    pub const fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    /// Number of waiting calls.
    #[must_use]
    pub const fn len(&self) -> usize {
        self.0.len()
    }

    /// Take every waiting call for one re-test pass.
    #[must_use]
    pub fn take_all(&mut self) -> Vec<WaitingCall> {
        core::mem::take(&mut self.0)
    }
}
