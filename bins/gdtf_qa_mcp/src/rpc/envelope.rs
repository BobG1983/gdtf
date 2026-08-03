//! Build JSON-RPC success and error response lines.

use serde_json::{Value, json};

/// The JSON-RPC protocol version every message carries.
const JSONRPC_VERSION: &str = "2.0";

/// Standard JSON-RPC error kinds we emit.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum RpcError {
    /// Invalid JSON.
    Parse,
    /// Missing required fields.
    InvalidRequest,
    /// Unknown method.
    MethodNotFound,
    /// Bad tool arguments.
    InvalidParams,
}

impl RpcError {
    const fn code(self) -> i64 {
        match self {
            Self::Parse => -32700,
            Self::InvalidRequest => -32600,
            Self::MethodNotFound => -32601,
            Self::InvalidParams => -32602,
        }
    }
}

fn to_line(message: &Value) -> String {
    serde_json::to_string(message).unwrap_or_default()
}

/// Success response line for `id` with `result`.
#[must_use]
pub(super) fn success_line(id: &Value, result: Value) -> String {
    to_line(&json!({
        "jsonrpc": JSONRPC_VERSION,
        "id": id,
        "result": result,
    }))
}

/// Error response line for `id`.
#[must_use]
pub(super) fn error_line(id: &Value, kind: RpcError, message: &str) -> String {
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

    #[test]
    fn success_line_echoes_id_and_result() {
        let line = success_line(&json!(7), json!({"ok": true}));
        let parsed: Value = serde_json::from_str(&line).unwrap_or(Value::Null);
        assert_eq!(parsed["jsonrpc"], json!("2.0"));
        assert_eq!(parsed["id"], json!(7));
        assert_eq!(parsed["result"], json!({"ok": true}));
    }

    #[test]
    fn error_line_carries_code_and_message() {
        let line = error_line(&Value::Null, RpcError::MethodNotFound, "nope");
        let parsed: Value = serde_json::from_str(&line).unwrap_or(Value::Null);
        assert_eq!(parsed["error"]["code"], json!(-32601));
        assert_eq!(parsed["error"]["message"], json!("nope"));
        assert_eq!(parsed["id"], Value::Null);
    }
}
