//! Route one run request: admit it, hold it while its budget lasts, or answer the refusal.

use bevy::{ecs::system::SystemParam, prelude::*};
use gdtf_net_qa_transport::Responder;
use gdtf_qa_protocol::command::{CommandArgsRon, CommandName, RunOptions};

use super::{
    Admission, CaptureHolds, CommandInbox, CommandRefusal, WaitingAge, WaitingCall, WaitingCalls,
    WaitingSince, admit, unavailable_reply, unknown_reply,
};
use crate::command::ErasedCommand;

/// One run request and the responder that answers it.
pub struct IncomingCall {
    name:      CommandName,
    arguments: CommandArgsRon,
    options:   RunOptions,
    responder: Responder,
}

impl IncomingCall {
    /// Build a call from a run request and its responder.
    #[must_use]
    pub const fn new(
        name: CommandName,
        arguments: CommandArgsRon,
        options: RunOptions,
        responder: Responder,
    ) -> Self {
        Self {
            name,
            arguments,
            options,
            responder,
        }
    }
}

/// The queues a router admits a call into, holds it in, or captures behind.
#[derive(SystemParam)]
pub struct CallQueues<'w> {
    inbox:   ResMut<'w, CommandInbox>,
    waiting: ResMut<'w, WaitingCalls>,
    holds:   ResMut<'w, CaptureHolds>,
}

impl CallQueues<'_> {
    fn admit_now(
        &mut self,
        name: CommandName,
        arguments: CommandArgsRon,
        options: RunOptions,
        responder: Responder,
    ) {
        let responder = match options.capture {
            Some(rider) => self.holds.hold(responder, rider.name),
            None => responder,
        };
        self.inbox.admit(name, arguments, responder);
    }

    fn settle(&mut self, call: WaitingCall, refusal: CommandRefusal, now: WaitingSince) {
        match call.age_at(now) {
            WaitingAge::Expired => call.into_responder().reply(unavailable_reply(refusal)),
            WaitingAge::Live => self.waiting.hold(call),
        }
    }
}

/// Decide one call now: admit it, hold it for a re-test, or answer the refusal.
pub fn route_call<F>(
    commands: &[&dyn ErasedCommand<F>],
    call: IncomingCall,
    facts: &F,
    queues: &mut CallQueues<'_>,
) {
    let IncomingCall {
        name,
        arguments,
        options,
        responder,
    } = call;
    let decision = admit(commands, &name, facts);
    match decision {
        Admission::Admit(command) => {
            queues.admit_now(command.name(), arguments, options, responder);
        }
        Admission::Unknown(known) => responder.reply(unknown_reply(known)),
        Admission::Unavailable(refusal) => match options.await_ready {
            None => responder.reply(unavailable_reply(refusal)),
            Some(budget) => {
                let held = WaitingCall::new(name, arguments, options, responder, budget);
                queues.settle(held, refusal, WaitingSince::now());
            }
        },
    }
}

/// Test every waiting call against the host's live facts again.
pub fn retest_waiting<F>(
    commands: &[&dyn ErasedCommand<F>],
    facts: &F,
    queues: &mut CallQueues<'_>,
) {
    if queues.waiting.is_empty() {
        return;
    }
    let now = WaitingSince::now();
    let held = queues.waiting.take_all();
    for call in held {
        let decision = admit(commands, call.name(), facts);
        match decision {
            Admission::Admit(_) => {
                let (name, arguments, options, responder) = call.into_parts();
                queues.admit_now(name, arguments, options, responder);
            }
            Admission::Unknown(known) => call.into_responder().reply(unknown_reply(known)),
            Admission::Unavailable(refusal) => queues.settle(call, refusal, now),
        }
    }
}
