//! JSON-RPC 2.0 envelope helpers — building the wire response / error lines (GTW-741).
//!
//! The MCP stdio transport is JSON-RPC 2.0: each message is one line of JSON. This
//! module turns a result / error into that line. The `id`, `method`, and version fields
//! are the external JSON-RPC protocol's own plumbing (mirrored as `serde_json::Value`,
//! not modelled as gdtf domain types).

use serde_json::{Value, json};

/// The JSON-RPC protocol version every message carries.
const JSONRPC_VERSION: &str = "2.0";

/// A JSON-RPC error **kind** — the standard error codes this bridge can return.
///
/// The four codes mirror the JSON-RPC 2.0 specification (external protocol constants,
/// not domain values). A typed enum keeps the call sites from juggling raw code numbers.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RpcError {
    /// The line was not valid JSON (`-32700`).
    Parse,
    /// A request object was missing a required member (`-32600`).
    InvalidRequest,
    /// The requested method is not implemented (`-32601`).
    MethodNotFound,
    /// The method's parameters were missing or invalid (`-32602`).
    InvalidParams,
}

impl RpcError {
    /// The JSON-RPC numeric code for this kind.
    const fn code(self) -> i64 {
        match self {
            Self::Parse => -32700,
            Self::InvalidRequest => -32600,
            Self::MethodNotFound => -32601,
            Self::InvalidParams => -32602,
        }
    }
}

/// Serialize a JSON-RPC message value to its single wire line.
///
/// `serde_json::to_string` on a `Value` never fails in practice; the empty-string
/// fallback keeps the function total without an `unwrap`.
fn to_line(message: &Value) -> String {
    serde_json::to_string(message).unwrap_or_default()
}

/// Build the wire line for a successful JSON-RPC response carrying `result`, echoing the
/// request `id`.
#[must_use]
pub fn success_line(id: &Value, result: Value) -> String {
    to_line(&json!({
        "jsonrpc": JSONRPC_VERSION,
        "id": id,
        "result": result,
    }))
}

/// Build the wire line for a JSON-RPC error response, echoing the request `id`.
#[must_use]
pub fn error_line(id: &Value, kind: RpcError, message: &str) -> String {
    to_line(&json!({
        "jsonrpc": JSONRPC_VERSION,
        "id": id,
        "error": { "code": kind.code(), "message": message },
    }))
}

#[cfg(test)]
mod tests {
    use serde_json::{Value, json};

    use super::{RpcError, error_line, success_line};

    /// A success line is well-formed JSON-RPC 2.0 and echoes the id verbatim.
    #[test]
    fn success_line_echoes_id_and_result() {
        let line = success_line(&json!(7), json!({"ok": true}));
        let parsed: Value = serde_json::from_str(&line).unwrap_or(Value::Null);
        assert_eq!(parsed["jsonrpc"], json!("2.0"));
        assert_eq!(parsed["id"], json!(7));
        assert_eq!(parsed["result"], json!({"ok": true}));
    }

    /// An error line carries the standard numeric code and message under `error`.
    #[test]
    fn error_line_carries_code_and_message() {
        let line = error_line(&Value::Null, RpcError::MethodNotFound, "nope");
        let parsed: Value = serde_json::from_str(&line).unwrap_or(Value::Null);
        assert_eq!(parsed["error"]["code"], json!(-32601));
        assert_eq!(parsed["error"]["message"], json!("nope"));
        assert_eq!(parsed["id"], Value::Null);
    }
}
