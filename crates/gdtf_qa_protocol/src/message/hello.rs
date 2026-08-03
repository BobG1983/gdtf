//! The handshake types — [`ProtocolVersion`], [`ServerNameNet`], [`HelloFacts`]
use bevy_derive::Deref;
use serde::{Deserialize, Serialize};

/// The wire **protocol version** — bumped on any breaking change to the message shapes.
/// A [`Hello`](crate::message::QaRequest::Hello) carries the client's version; the
#[derive(Deref, Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct ProtocolVersion(u32);

impl ProtocolVersion {
                                            /// Both are `#[serde(default)]`, so a NEW decoder reads an OLD frame with its previous
                                                                        pub const CURRENT: Self = Self::new(14);

        #[must_use]
    pub const fn new(version: u32) -> Self {
        Self(version)
    }
}

#[derive(Deref, Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct ServerNameNet(String);

impl ServerNameNet {
        #[must_use]
    pub const fn new(name: String) -> Self {
        Self(name)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct HelloFacts {
        pub protocol: ProtocolVersion,
        pub server:   ServerNameNet,
}

impl HelloFacts {
        #[must_use]
    pub const fn new(protocol: ProtocolVersion, server: ServerNameNet) -> Self {
        Self { protocol, server }
    }
}
