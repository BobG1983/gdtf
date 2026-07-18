//! [`handle_tool_call`] — resolve the tool, then either drive the game lifecycle (the
//! two host-local tools) or build the game request, carry it over the link, and render
//! the reply.

use serde_json::{Value, json};

use super::{build::build_request, render::render_response};
use crate::{
    game::GameLink,
    lifecycle::GameLifecycle,
    mcp::{content::tool_error, control, tools::ToolName},
};

/// The outcome of handling a `tools/call` — either a JSON-RPC `result` object (which may
/// itself carry an MCP tool error), or an invalid-params rejection the caller renders as a
/// JSON-RPC error.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ToolCallOutcome {
    /// A `tools/call` result object (an MCP content block).
    Result(Value),
    /// The call parameters were missing or invalid; the message explains why.
    Invalid(String),
}

/// Handle a `tools/call` request: resolve the tool, then either drive the game lifecycle
/// (the two host-local tools) or build the game request, carry it over the link, and
/// render the reply.
///
/// A missing `params`, a missing / unknown tool name, or an un-buildable request is an
/// [`Invalid`](ToolCallOutcome::Invalid) (JSON-RPC invalid-params). A link or lifecycle
/// failure is a tool error inside a normal result.
#[must_use]
pub fn handle_tool_call(
    params: Option<&Value>,
    game: &mut dyn GameLink,
    lifecycle: &mut dyn GameLifecycle,
) -> ToolCallOutcome {
    let Some(params) = params else {
        return ToolCallOutcome::Invalid("`tools/call` needs `params`".to_owned());
    };
    let Some(name) = params.get("name").and_then(Value::as_str) else {
        return ToolCallOutcome::Invalid("`tools/call` needs a tool `name`".to_owned());
    };
    let Some(tool) = ToolName::from_wire(name) else {
        return ToolCallOutcome::Invalid(format!("unknown tool: {name}"));
    };
    let empty = json!({});
    let args = params.get("arguments").unwrap_or(&empty);
    match tool {
        // The two host-local tools start / stop the game process rather than forwarding a
        // request to a running one.
        ToolName::LaunchGame => control::handle_launch(args, game, lifecycle),
        ToolName::StopGame => control::handle_stop(lifecycle),
        // The seven forwarding tools map onto a `QaRequest` carried over the link.
        _ => match build_request(tool, args) {
            Ok(request) => match game.request(request) {
                Ok(response) => ToolCallOutcome::Result(render_response(tool, &response)),
                Err(err) => ToolCallOutcome::Result(tool_error(&err.to_string())),
            },
            Err(message) => ToolCallOutcome::Invalid(message),
        },
    }
}
