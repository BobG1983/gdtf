use bevy::prelude::*;
use gdtf_net_qa_transport::PendingQueue;
use gdtf_qa_protocol::{
    command::{ArgSchemaJson, ArgumentFault, CommandOutcome},
    message::QaResponse,
};

use super::{CommandCall, CommandInbox};
use crate::command::{QaCommand, schema::schema_text};

#[must_use]
pub fn bad_arguments<C: QaCommand>(fault: &serde_json::Error) -> QaResponse {
    QaResponse::Outcome(CommandOutcome::BadArguments {
        detail: ArgumentFault::new(fault.to_string()),
        schema: ArgSchemaJson::new(schema_text::<C::Args>()),
    })
}

pub fn claim_calls<C: QaCommand>(
    mut inbox: ResMut<CommandInbox>,
    mut queue: ResMut<PendingQueue<CommandCall<C>>>,
) {
    if inbox.is_empty() {
        return;
    }
    for (arguments, responder) in inbox.take_for(&C::NAME) {
        match serde_json::from_str::<C::Args>(arguments.as_str()) {
            Ok(args) => queue.push_new(CommandCall::<C>::new(args), responder),
            Err(fault) => responder.reply(bad_arguments::<C>(&fault)),
        }
    }
}
