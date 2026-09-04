//! Protocol replies for unavailable and unknown commands.

use cobalt_mcp_protocol::{
    command::{CommandName, CommandOutcome},
    message::QaResponse,
};

use super::CommandRefusal;

/// Outcome reply when a known command was refused.
#[must_use]
pub fn unavailable_reply(refusal: CommandRefusal) -> QaResponse {
    let (code, note) = refusal.into_parts();
    QaResponse::Outcome(CommandOutcome::Unavailable { code, note })
}

/// Outcome reply when the command name is not in the set.
#[must_use]
pub const fn unknown_reply(known: Vec<CommandName>) -> QaResponse {
    QaResponse::Outcome(CommandOutcome::Unknown { known })
}
