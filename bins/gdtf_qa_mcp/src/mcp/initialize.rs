//! MCP `initialize` response.

use serde_json::{Value, json};

/// The MCP protocol version advertised when the client does not request one.
const DEFAULT_MCP_PROTOCOL_VERSION: &str = "2024-11-05";

const SERVER_NAME: &str = "gdtf-qa-mcp";

const SERVER_VERSION: &str = env!("CARGO_PKG_VERSION");

/// Build the initialize result, echoing the client's protocol version when present.
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

    #[test]
    fn echoes_requested_protocol_version() {
        let params = json!({"protocolVersion": "2025-06-18"});
        let result = initialize_result(Some(&params));
        assert_eq!(result["protocolVersion"], json!("2025-06-18"));
    }

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
