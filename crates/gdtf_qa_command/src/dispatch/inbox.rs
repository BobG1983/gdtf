//! [`CommandInbox`] — the calls this frame's router admitted, waiting to be claimed.

use bevy::prelude::*;
use gdtf_net_qa_transport::Responder;
use gdtf_qa_protocol::command::{CommandArgsJson, CommandName};

/// One admitted call: which command it is for, its still-undecoded arguments, and the
/// channel it will be answered on.
///
/// The arguments stay JSON here on purpose — the inbox is the last place in the path that
/// is generic over every command, and only the command's own `Args` type gives them
/// meaning. [`claim_calls`](super::claim_calls) is where they stop being text.
pub struct AdmittedCall {
    /// The command the call is addressed to.
    name:      CommandName,
    /// The call's arguments, still as JSON text.
    arguments: CommandArgsJson,
    /// The one-shot reply channel back to the socket.
    responder: Responder,
}

impl AdmittedCall {
    /// The command this call is addressed to.
    #[must_use]
    pub const fn name(&self) -> &CommandName {
        &self.name
    }
}

/// The calls this frame's router admitted, waiting to be claimed by their command.
///
/// One resource for every command on the host — the decode steps take it in turn (Bevy
/// serialises them, since each takes `ResMut` of it), and each takes only the calls
/// addressed to its own name.
#[derive(Resource, Default)]
pub struct CommandInbox(Vec<AdmittedCall>);

impl CommandInbox {
    /// Park an admitted call.
    pub fn admit(&mut self, name: CommandName, arguments: CommandArgsJson, responder: Responder) {
        self.0.push(AdmittedCall {
            name,
            arguments,
            responder,
        });
    }

    /// Whether anything is waiting.
    ///
    /// The decode steps' early-out. It takes `&self`, so reading it through a `ResMut` does
    /// NOT dirty change detection — which matters because a host with fifty commands runs
    /// fifty of these checks a frame and the inbox is empty on essentially every one.
    #[must_use]
    pub const fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    /// How many calls are waiting.
    #[must_use]
    pub const fn len(&self) -> usize {
        self.0.len()
    }

    /// Take every call addressed to `name`, leaving the rest in place and in order.
    #[must_use]
    pub fn take_for(&mut self, name: &CommandName) -> Vec<(CommandArgsJson, Responder)> {
        let mut taken = Vec::new();
        let mut kept = Vec::with_capacity(self.0.len());
        for call in self.0.drain(..) {
            if call.name == *name {
                taken.push((call.arguments, call.responder));
            } else {
                kept.push(call);
            }
        }
        self.0 = kept;
        taken
    }
}
