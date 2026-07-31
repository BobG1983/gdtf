//! The decode step — where a command's admitted calls stop being JSON.

use bevy::prelude::*;
use gdtf_net_qa_transport::PendingQueue;
use gdtf_qa_protocol::{
    command::{ArgSchemaJson, ArgumentFault, CommandOutcome},
    envelope::QaResponse,
};

use super::{CommandCall, CommandInbox};
use crate::command::{QaCommand, schema::schema_text};

/// The `BadArguments` answer for a payload that will not decode into `C::Args`.
///
/// The schema attached is `C`'s OWN derived argument schema — the same document the
/// catalogue publishes, from the same derivation — so the app tells the caller what it
/// should have sent instead of leaving it to guess, and cannot describe a shape it does
/// not accept.
#[must_use]
pub fn bad_arguments<C: QaCommand>(fault: &serde_json::Error) -> QaResponse {
    QaResponse::Outcome(CommandOutcome::BadArguments {
        detail: ArgumentFault::new(fault.to_string()),
        schema: ArgSchemaJson::new(schema_text::<C::Args>()),
    })
}

/// Decode this command's admitted calls into its typed queue.
///
/// One instantiation per command, registered by
/// [`register_command`](super::register_command) in
/// [`QaCommandSystems::Claim`](super::QaCommandSystems::Claim). A payload that does not
/// decode is answered HERE and never reaches the handler, so a handler only ever sees a
/// well-formed `C::Args`.
pub fn claim_calls<C: QaCommand>(
    mut inbox: ResMut<CommandInbox>,
    mut queue: ResMut<PendingQueue<CommandCall<C>>>,
) {
    // Read emptiness through the immutable accessor so an idle frame never dirties either
    // resource's change-detection flag.
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
