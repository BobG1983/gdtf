use gdtf_qa_protocol::{
    command::{CommandName, CommandOutcome},
    message::QaResponse,
};

use super::CommandRefusal;

#[must_use]
pub fn unavailable_reply(refusal: CommandRefusal) -> QaResponse {
    let (code, note) = refusal.into_parts();
    QaResponse::Outcome(CommandOutcome::Unavailable { code, note })
}

#[must_use]
pub const fn unknown_reply(known: Vec<CommandName>) -> QaResponse {
    QaResponse::Outcome(CommandOutcome::Unknown { known })
}
