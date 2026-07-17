//! The handshake types — [`ProtocolVersion`], [`ServerNameNet`], [`HelloFacts`]
//! (GTW-734).

use bevy_derive::Deref;
use serde::{Deserialize, Serialize};

/// The wire **protocol version** — bumped on any breaking envelope change.
///
/// A [`Hello`](crate::envelope::QaRequest::Hello) carries the client's version; the
/// server replies [`HelloOk`](crate::envelope::QaResponse::HelloOk) on a match or
/// [`VersionMismatch`](crate::envelope::QaError::VersionMismatch) otherwise. A private-
/// inner newtype (no-bare-types), serde-transparent over `u32`.
#[derive(Deref, Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct ProtocolVersion(u32);

impl ProtocolVersion {
    /// The wire protocol version this build of the contract speaks.
    ///
    /// Bumped to `2` when [`AppFlowView`](crate::view::AppFlowView) gained its `available`
    /// affordance list (GTW-746) — an existing wire shape changed, so a client negotiating
    /// the old version `1` now gets a
    /// [`VersionMismatch`](crate::envelope::QaError::VersionMismatch) rather than a snapshot
    /// missing the field. The game server negotiates a `Hello` against this value.
    pub const CURRENT: Self = Self::new(2);

    /// Build a protocol version from its number.
    #[must_use]
    pub const fn new(version: u32) -> Self {
        Self(version)
    }
}

/// The server's self-identifying **name** returned in the handshake (e.g. the game
/// build id) — so a client can log what it connected to.
///
/// A name newtype over `String` (no-bare-types), serde-transparent. `Clone`-not-`Copy`.
#[derive(Deref, Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct ServerNameNet(String);

impl ServerNameNet {
    /// Build a server name from its identifying string.
    #[must_use]
    pub const fn new(name: String) -> Self {
        Self(name)
    }
}

/// The facts the server hands back on a successful handshake — the negotiated
/// [`protocol`](Self::protocol) version and the [`server`](Self::server) identity.
///
/// The [`HelloOk`](crate::envelope::QaResponse::HelloOk) payload. A struct (not a bare
/// version) so the handshake can grow more negotiated facts without a wire break. Serde
/// default shape.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct HelloFacts {
    /// The protocol version the server speaks.
    pub protocol: ProtocolVersion,
    /// The server's self-identifying name.
    pub server:   ServerNameNet,
}

impl HelloFacts {
    /// Build the handshake facts from the protocol version and server name.
    #[must_use]
    pub const fn new(protocol: ProtocolVersion, server: ServerNameNet) -> Self {
        Self { protocol, server }
    }
}
