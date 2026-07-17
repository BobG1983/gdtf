//! MCP tool-result content blocks — the shapes a `tools/call` reply carries (GTW-741,
//! GTW-745).
//!
//! Every tool renders its reply as one of these blocks: `text_content` for a JSON
//! payload, `image_content` for a screenshot, or `tool_error` for a failure
//! (`isError: true`). Shared by the forwarding-tool handler in [`call`](super::call) and
//! the lifecycle-tool handlers in [`control`](super::control).

use serde::Serialize;
use serde_json::{Value, json};

/// A pretty-JSON text content block (the successful, non-image tool payload).
pub(super) fn text_content<T: Serialize>(value: &T) -> Value {
    let text =
        serde_json::to_string_pretty(value).unwrap_or_else(|_| "<unserializable>".to_owned());
    json!({ "content": [ { "type": "text", "text": text } ], "isError": false })
}

/// A base64 PNG image content block.
pub(super) fn image_content(data: &str) -> Value {
    json!({
        "content": [ { "type": "image", "data": data, "mimeType": "image/png" } ],
        "isError": false
    })
}

/// A tool-error content block (`isError: true`) carrying a human-readable reason.
pub(super) fn tool_error(message: &str) -> Value {
    json!({ "content": [ { "type": "text", "text": message } ], "isError": true })
}
