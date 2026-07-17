//! The `initialize` handshake result (GTW-741).
//!
//! MCP opens with an `initialize` call; the server answers with the protocol version it
//! speaks, its capabilities, and its identity. This bridge advertises exactly one
//! capability — `tools` — and echoes the client's requested protocol version when the
//! client sends one (a permissive subset), falling back to a known default otherwise.

use serde_json::{Value, json};

/// The MCP protocol version advertised when the client does not request one.
const DEFAULT_MCP_PROTOCOL_VERSION: &str = "2024-11-05";

/// This server's self-identifying name in the handshake.
const SERVER_NAME: &str = "gdtf-qa-mcp";

/// This server's version — the crate version at build time.
const SERVER_VERSION: &str = env!("CARGO_PKG_VERSION");

/// Build the `initialize` result object.
///
/// Echoes the `protocolVersion` string from `params` when present (so the negotiated
/// version matches whatever the client asked for), else advertises
/// `DEFAULT_MCP_PROTOCOL_VERSION`. Advertises the `tools` capability and the server
/// identity.
#[must_use]
pub fn initialize_result(params: Option<&Value>) -> Value {
    let protocol = params
        .and_then(|value| value.get("protocolVersion"))
        .and_then(Value::as_str)
        .unwrap_or(DEFAULT_MCP_PROTOCOL_VERSION);
    json!({
        "protocolVersion": protocol,
        "capabilities": { "tools": {} },
        "serverInfo": { "name": SERVER_NAME, "version": SERVER_VERSION },
    })
}

#[cfg(test)]
mod tests {
    use serde_json::{Value, json};

    use super::{DEFAULT_MCP_PROTOCOL_VERSION, initialize_result};

    /// A requested protocol version is echoed back.
    #[test]
    fn echoes_requested_protocol_version() {
        let params = json!({"protocolVersion": "2025-06-18"});
        let result = initialize_result(Some(&params));
        assert_eq!(result["protocolVersion"], json!("2025-06-18"));
    }

    /// With no requested version, the default is advertised, along with the tools
    /// capability.
    #[test]
    fn falls_back_to_default_and_advertises_tools() {
        let result = initialize_result(None);
        assert_eq!(
            result["protocolVersion"],
            json!(DEFAULT_MCP_PROTOCOL_VERSION)
        );
        assert!(result["capabilities"].get("tools").is_some());
        assert_ne!(result["serverInfo"]["name"], Value::Null);
    }
}
