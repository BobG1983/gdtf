//! Route one JSON-RPC request line to MCP handlers.

use serde_json::Value;

use super::envelope::{RpcError, error_line, success_line};
use crate::{
    hosts::HostSet,
    mcp::{ToolCallOutcome, handle_tool_call, initialize_result, tools_list_result},
};

/// Parse and handle one request line; `None` for notifications / empty lines.
#[must_use]
pub fn dispatch(line: &str, hosts: &mut HostSet<'_>) -> Option<String> {
    let trimmed = line.trim();
    if trimmed.is_empty() {
        return None;
    }
    let Ok(request) = serde_json::from_str::<Value>(trimmed) else {
        return Some(error_line(&Value::Null, RpcError::Parse, "invalid JSON"));
    };
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
        "tools/call" => match handle_tool_call(params, hosts) {
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
