//! Shared inbox of admitted but not-yet-claimed calls.

use bevy::prelude::*;
use cobalt_mcp_protocol::command::{CommandArgsRon, CommandName};

use crate::transport::Responder;

/// One admitted call waiting to be claimed by its command system.
pub struct AdmittedCall {
    name:      CommandName,
    arguments: CommandArgsRon,
    responder: Responder,
}

impl AdmittedCall {
    /// Command name this call targets.
    #[must_use]
    pub const fn name(&self) -> &CommandName {
        &self.name
    }
}

/// Bevy resource holding admitted calls until claim systems drain them.
#[derive(Resource, Default)]
pub struct CommandInbox(Vec<AdmittedCall>);

impl CommandInbox {
    /// Push a new admitted call.
    pub fn admit(&mut self, name: CommandName, arguments: CommandArgsRon, responder: Responder) {
        self.0.push(AdmittedCall {
            name,
            arguments,
            responder,
        });
    }

    /// Whether the inbox is empty.
    #[must_use]
    pub const fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    /// Number of pending calls.
    #[must_use]
    pub const fn len(&self) -> usize {
        self.0.len()
    }

    /// Take all calls matching `name` and leave the rest.
    #[must_use]
    pub fn take_for(&mut self, name: &CommandName) -> Vec<(CommandArgsRon, Responder)> {
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
