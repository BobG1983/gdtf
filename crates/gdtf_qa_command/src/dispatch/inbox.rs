use bevy::prelude::*;
use gdtf_net_qa_transport::Responder;
use gdtf_qa_protocol::command::{CommandArgsJson, CommandName};

pub struct AdmittedCall {
        name:      CommandName,
        arguments: CommandArgsJson,
        responder: Responder,
}

impl AdmittedCall {
        #[must_use]
    pub const fn name(&self) -> &CommandName {
        &self.name
    }
}

#[derive(Resource, Default)]
pub struct CommandInbox(Vec<AdmittedCall>);

impl CommandInbox {
        pub fn admit(&mut self, name: CommandName, arguments: CommandArgsJson, responder: Responder) {
        self.0.push(AdmittedCall {
            name,
            arguments,
            responder,
        });
    }

                        #[must_use]
    pub const fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

        #[must_use]
    pub const fn len(&self) -> usize {
        self.0.len()
    }

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
