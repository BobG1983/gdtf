use cobalt_mcp_protocol::{command::CommandOutcome, message::QaResponse};

use crate::support::TestError;

/// The detail a run answered `BadArguments` with, or why the reply was not that refusal.
pub(crate) fn bad_arguments_detail(reply: &QaResponse) -> Result<String, TestError> {
    let QaResponse::Outcome(CommandOutcome::BadArguments { detail, schema }) = reply else {
        return Err(format!("expected a BadArguments outcome, got {reply:?}").into());
    };
    if schema.as_str().is_empty() {
        return Err(
            "a BadArguments carries the shape a client may send, and this one is empty"
                .to_owned()
                .into(),
        );
    }
    Ok(detail.as_str().to_owned())
}
