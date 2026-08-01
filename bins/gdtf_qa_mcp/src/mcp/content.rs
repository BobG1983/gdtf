//! MCP tool-result content blocks — the shapes a `tools/call` reply carries (GTW-741,
//! GTW-745).
//!
//! Every tool renders its reply as one of these blocks: `text_content` for a JSON payload
//! or `tool_error` for a failure (`isError: true`). Shared by the courier's dispatch and the
//! host-local handlers in [`control`](super::control). An IMAGE is not here: a capture comes
//! back as an attachment on a command's reply, and the courier's `attach` module builds that
//! block beside the reply's own text rather than replacing it.

use serde::Serialize;
use serde_json::{Value, json};

/// A pretty-JSON text content block (the successful, non-image tool payload).
pub(super) fn text_content<T: Serialize>(value: &T) -> Value {
    let text =
        serde_json::to_string_pretty(value).unwrap_or_else(|_| "<unserializable>".to_owned());
    json!({ "content": [ { "type": "text", "text": text } ], "isError": false })
}

/// A tool-error content block (`isError: true`) carrying a human-readable reason.
pub(super) fn tool_error(message: &str) -> Value {
    json!({ "content": [ { "type": "text", "text": message } ], "isError": true })
}
