use cobalt_mcp_protocol::{
    command::{CommandAvailability, CommandName, CommandOutcome},
    message::McpResponse,
};

use crate::support::TestError;

/// The refusal code and note a run answered with, or why the reply was not a refusal.
pub(crate) fn refusal_of(reply: &McpResponse) -> Result<(String, String), TestError> {
    let McpResponse::Outcome(CommandOutcome::Unavailable { code, note }) = reply else {
        return Err(format!("expected an Unavailable outcome, got {reply:?}").into());
    };
    Ok((format!("{code:?}"), note.as_str().to_owned()))
}

/// The refusal code and note the catalogue publishes for one command.
pub(crate) fn catalogue_refusal_of(
    reply: &McpResponse,
    command: &'static str,
) -> Result<(String, String), TestError> {
    let McpResponse::Catalogue(catalogue) = reply else {
        return Err(format!("expected a Catalogue reply, got {reply:?}").into());
    };
    let Some(entry) = catalogue
        .entries
        .iter()
        .find(|entry| entry.command == CommandName::from_static(command))
    else {
        return Err(format!("the catalogue carries no row for `{command}`: {catalogue:?}").into());
    };
    let CommandAvailability::Unavailable { code, note } = &entry.availability else {
        return Err(format!(
            "expected `{command}` to be refused in this phase, got {:?}",
            entry.availability
        )
        .into());
    };
    Ok((format!("{code:?}"), note.as_str().to_owned()))
}
