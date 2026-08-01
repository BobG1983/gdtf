//! The two route-time reply shapers a host's router answers a non-admitted call with.

use gdtf_qa_protocol::{
    command::{CommandName, CommandOutcome},
    message::QaResponse,
};

use super::CommandRefusal;

/// The reply for a call whose command exists but is not admissible right now.
///
/// Total by construction: [`CommandRefusal`] cannot hold the `Available` case, so a host's
/// router never writes an arm for a state that cannot occur.
#[must_use]
pub fn unavailable_reply(refusal: CommandRefusal) -> QaResponse {
    let (code, note) = refusal.into_parts();
    QaResponse::Outcome(CommandOutcome::Unavailable { code, note })
}

/// The reply for a name this host does not offer, carrying every name it does.
#[must_use]
pub const fn unknown_reply(known: Vec<CommandName>) -> QaResponse {
    QaResponse::Outcome(CommandOutcome::Unknown { known })
}
