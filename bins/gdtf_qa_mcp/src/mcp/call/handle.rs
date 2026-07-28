//! [`handle_tool_call`] — resolve the tool, pick the host it acts on, then either drive
//! that host's lifecycle (the four launch / stop tools) or build the request, carry it
//! over that host's link, and render the reply.

use serde_json::{Value, json};

use super::{build::build_request, render::render_response};
use crate::{
    hosts::HostSet,
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

/// Handle a `tools/call` request: resolve the tool, take the link + lifecycle belonging to
/// the host it names, then either drive that host's lifecycle (the four launch / stop
/// tools) or build the request, carry it over that host's link, and render the reply.
///
/// The host comes from [`ToolName::host`], so an editor tool never travels over the game's
/// link and the two children are driven independently — which is what lets a game and an
/// editor be up at the same time (GTW-808).
///
/// A missing `params`, a missing / unknown tool name, or an un-buildable request is an
/// [`Invalid`](ToolCallOutcome::Invalid) (JSON-RPC invalid-params). A link or lifecycle
/// failure is a tool error inside a normal result.
#[must_use]
pub fn handle_tool_call(params: Option<&Value>, hosts: &mut HostSet<'_>) -> ToolCallOutcome {
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
    let host = tool.host();
    let pair = hosts.pair(host);
    if tool.is_launch() {
        // A launch starts that host's process rather than forwarding a request to it.
        let (link, lifecycle) = pair.parts();
        return control::handle_launch(host, args, link, lifecycle);
    }
    if tool.is_stop() {
        return control::handle_stop(pair.lifecycle());
    }
    // Every other tool maps onto a `QaRequest` carried over its host's link.
    match build_request(tool, args) {
        Ok(request) => match pair.link().request(request) {
            Ok(response) => ToolCallOutcome::Result(render_response(tool, &response)),
            Err(err) => ToolCallOutcome::Result(tool_error(&err.to_string())),
        },
        Err(message) => ToolCallOutcome::Invalid(message),
    }
}
