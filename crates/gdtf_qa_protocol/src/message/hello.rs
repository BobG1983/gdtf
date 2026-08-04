//! Handshake types: protocol version, server name, hello facts.

use bevy_derive::Deref;
use serde::{Deserialize, Serialize};

/// Wire protocol version. Bump on breaking message shape changes.
/// A [`Hello`](crate::message::QaRequest::Hello) carries the client's version.
#[derive(Deref, Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct ProtocolVersion(u32);

impl ProtocolVersion {
    /// Current protocol version implemented by this crate.
    pub const CURRENT: Self = Self::new(14);

    /// Wrap a version number.
    #[must_use]
    pub const fn new(version: u32) -> Self {
        Self(version)
    }
}

/// Server display name on the wire.
#[derive(Deref, Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct ServerNameNet(String);

impl ServerNameNet {
    /// Wrap an owned server name.
    #[must_use]
    pub const fn new(name: String) -> Self {
        Self(name)
    }
}

/// Facts returned in a successful hello response.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct HelloFacts {
    /// Server protocol version.
    pub protocol: ProtocolVersion,
    /// Server name.
    pub server:   ServerNameNet,
}

impl HelloFacts {
    /// Build hello facts from version and name.
    #[must_use]
    pub const fn new(protocol: ProtocolVersion, server: ServerNameNet) -> Self {
        Self { protocol, server }
    }
}
