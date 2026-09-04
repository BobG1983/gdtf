//! Move inbox entries for one command into its typed pending queue.

use bevy::prelude::*;
use cobalt_mcp_protocol::{
    command::{ArgSchemaRon, ArgumentFault, CommandOutcome, shape_text},
    message::QaResponse,
};
use cobalt_mcp_transport::PendingQueue;

use super::{CommandCall, CommandInbox};
use crate::command::QaCommand;

/// Build a bad-arguments outcome that includes the expected shape.
#[must_use]
pub fn bad_arguments<C: QaCommand>(fault: &ron::error::SpannedError) -> QaResponse {
    QaResponse::Outcome(CommandOutcome::BadArguments {
        detail: ArgumentFault::new(fault.to_string()),
        schema: ArgSchemaRon::new(shape_text::<C::Args>()),
    })
}

/// Claim inbox rows for `C`, deserialize args, or reply with a shape error.
pub fn claim_calls<C: QaCommand>(
    mut inbox: ResMut<CommandInbox>,
    mut queue: ResMut<PendingQueue<CommandCall<C>>>,
) {
    if inbox.is_empty() {
        return;
    }
    for (arguments, responder) in inbox.take_for(&C::NAME) {
        match ron::de::from_str::<C::Args>(arguments.as_str()) {
            Ok(args) => queue.push_new(CommandCall::<C>::new(args), responder),
            Err(fault) => responder.reply(bad_arguments::<C>(&fault)),
        }
    }
}
