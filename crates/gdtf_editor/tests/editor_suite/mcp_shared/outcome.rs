use cobalt_mcp_protocol::{command::CommandOutcome, message::McpResponse};
use serde::de::DeserializeOwned;

use crate::mcp_shared::support::TestError;

/// Decode the RON body of a `Ran` outcome, or say which outcome came back instead.
pub(crate) fn ran_body<T: DeserializeOwned>(
    reply: &McpResponse,
    command: &str,
) -> Result<T, TestError> {
    let McpResponse::Outcome(CommandOutcome::Ran { reply: body, .. }) = reply else {
        return Err(format!("expected a Ran outcome for `{command}`, got {reply:?}").into());
    };
    Ok(ron::de::from_str::<T>(body.as_str())?)
}

/// The refusal code a run answered with, or why the reply was not a refusal.
pub(crate) fn unavailable_code(reply: &McpResponse) -> Result<String, TestError> {
    let McpResponse::Outcome(CommandOutcome::Unavailable { code, .. }) = reply else {
        return Err(format!("expected an Unavailable outcome, got {reply:?}").into());
    };
    Ok(format!("{code:?}"))
}
