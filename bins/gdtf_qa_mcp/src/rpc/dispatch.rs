//! The JSON-RPC method router — one request line in, one response line out (GTW-741).
//!
//! [`dispatch`] is the whole MCP surface: it parses a JSON-RPC line, routes the method to
//! its handler, and returns the response line (or `None` for a notification / blank line,
//! which JSON-RPC answers with silence). The three contract methods —
//! [`initialize`](crate::mcp::initialize_result),
//! [`tools/list`](crate::mcp::tools_list_result), and
//! [`tools/call`](crate::mcp::handle_tool_call) — are handled here; `ping` is answered
//! with an empty result and any other method with `MethodNotFound`.

use serde_json::Value;

use super::envelope::{RpcError, error_line, success_line};
use crate::{
    game::GameLink,
    mcp::{ToolCallOutcome, handle_tool_call, initialize_result, tools_list_result},
};

/// Route one JSON-RPC request line and produce its response line.
///
/// Returns `None` when there is nothing to answer: a blank line, or a JSON-RPC
/// notification (a message with no `id`), which the spec answers with silence. `game` is
/// the link a `tools/call` reaches through; `initialize` / `tools/list` never touch it.
#[must_use]
pub fn dispatch(line: &str, game: &mut dyn GameLink) -> Option<String> {
    let trimmed = line.trim();
    if trimmed.is_empty() {
        return None;
    }
    let Ok(request) = serde_json::from_str::<Value>(trimmed) else {
        return Some(error_line(&Value::Null, RpcError::Parse, "invalid JSON"));
    };
    // No `id` member ⇒ a notification: answer nothing (`?` yields `None` from here).
    let id = request.get("id").cloned()?;
    let Some(method) = request.get("method").and_then(Value::as_str) else {
        return Some(error_line(
            &id,
            RpcError::InvalidRequest,
            "missing `method`",
        ));
    };
    let params = request.get("params");
    let line = match method {
        "initialize" => success_line(&id, initialize_result(params)),
        "tools/list" => success_line(&id, tools_list_result()),
        "tools/call" => match handle_tool_call(params, game) {
            ToolCallOutcome::Result(result) => success_line(&id, result),
            ToolCallOutcome::Invalid(message) => error_line(&id, RpcError::InvalidParams, &message),
        },
        "ping" => success_line(&id, Value::Object(serde_json::Map::new())),
        other => error_line(
            &id,
            RpcError::MethodNotFound,
            &format!("method not found: {other}"),
        ),
    };
    Some(line)
}
