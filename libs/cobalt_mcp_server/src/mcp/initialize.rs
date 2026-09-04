//! MCP `initialize` response.

use core::ops::Deref;

use serde_json::{Value, json};

/// The MCP protocol version advertised when the client does not request one.
const DEFAULT_MCP_PROTOCOL_VERSION: &str = "2024-11-05";

/// Name this server advertises to a client.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ServerName(String);

impl ServerName {
    /// Wrap an advertised name.
    #[must_use]
    pub const fn new(name: String) -> Self {
        Self(name)
    }
}

impl Deref for ServerName {
    type Target = str;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

/// Version this server advertises to a client.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ServerVersion(String);

impl ServerVersion {
    /// Wrap an advertised version.
    #[must_use]
    pub const fn new(version: String) -> Self {
        Self(version)
    }
}

impl Deref for ServerVersion {
    type Target = str;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

/// What a client is told this server is when it connects.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ServerIdentity {
    name:    ServerName,
    version: ServerVersion,
}

impl ServerIdentity {
    /// Advertise this name and version.
    #[must_use]
    pub const fn new(name: ServerName, version: ServerVersion) -> Self {
        Self { name, version }
    }

    /// Advertised name.
    #[must_use]
    pub const fn name(&self) -> &ServerName {
        &self.name
    }

    /// Advertised version.
    #[must_use]
    pub const fn version(&self) -> &ServerVersion {
        &self.version
    }
}

/// Build the initialize result, echoing the client's protocol version when present.
#[must_use]
pub fn initialize_result(identity: &ServerIdentity, params: Option<&Value>) -> Value {
    let protocol = params
        .and_then(|value| value.get("protocolVersion"))
        .and_then(Value::as_str)
        .unwrap_or(DEFAULT_MCP_PROTOCOL_VERSION);
    json!({
        "protocolVersion": protocol,
        "capabilities": { "tools": {} },
        "serverInfo": { "name": &*identity.name, "version": &*identity.version },
    })
}

#[cfg(test)]
mod tests {
    use serde_json::{Value, json};

    use super::{
        DEFAULT_MCP_PROTOCOL_VERSION, ServerIdentity, ServerName, ServerVersion, initialize_result,
    };

    fn identity() -> ServerIdentity {
        ServerIdentity::new(
            ServerName::new("sample-bridge".to_owned()),
            ServerVersion::new("9.9.9".to_owned()),
        )
    }

    #[test]
    fn echoes_requested_protocol_version() {
        let params = json!({"protocolVersion": "2025-06-18"});
        let result = initialize_result(&identity(), Some(&params));
        assert_eq!(result["protocolVersion"], json!("2025-06-18"));
    }

    #[test]
    fn advertises_the_identity_the_caller_supplied() {
        let result = initialize_result(&identity(), None);
        assert_eq!(
            result["protocolVersion"],
            json!(DEFAULT_MCP_PROTOCOL_VERSION)
        );
        assert!(result["capabilities"].get("tools").is_some());
        assert_eq!(result["serverInfo"]["name"], json!("sample-bridge"));
        assert_eq!(result["serverInfo"]["version"], json!("9.9.9"));
        assert_ne!(result["serverInfo"]["name"], Value::Null);
    }
}
