//! The handshake types — [`ProtocolVersion`], [`ServerNameNet`], [`HelloFacts`]
//! (GTW-734).

use bevy_derive::Deref;
use serde::{Deserialize, Serialize};

/// The wire **protocol version** — bumped on any breaking change to the message shapes.
///
/// A [`Hello`](crate::message::QaRequest::Hello) carries the client's version; the
/// server replies [`HelloOk`](crate::message::QaResponse::HelloOk) on a match or
/// [`VersionMismatch`](crate::message::QaError::VersionMismatch) otherwise. A private-
/// inner newtype (no-bare-types), serde-transparent over `u32`.
#[derive(Deref, Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct ProtocolVersion(u32);

impl ProtocolVersion {
    /// The wire protocol version this build of the contract speaks.
    ///
    /// Negotiation is exact equality with no capability handshake, so ANY change to a
    /// shape either side decodes has to move this number: a peer left on the old value
    /// must be refused outright rather than allowed to negotiate and then mis-decode.
    ///
    /// It reached `14` in GTW-942, when [`RunCommand`](crate::message::RunCommand) gained
    /// `options` (the per-call riders, so a rider a client wrote actually reaches the host
    /// that must refuse or honour it) and [`CommandEntry`](crate::command::CommandEntry)
    /// gained `timing` (whether a command answers on the claiming frame or a later one).
    /// Both are `#[serde(default)]`, so a NEW decoder reads an OLD frame with its previous
    /// meaning — but the other direction loses data silently, and moving the number is what
    /// stops those two builds from negotiating at all.
    ///
    /// GTW-943 then deleted the whole pre-command request vocabulary and left the number
    /// where it was. A deletion is as breaking as an addition, but nothing had negotiated
    /// `14` before that landing, so no peer can be holding the shape that was cut. The
    /// twelve bumps before it belong to a wire that no longer exists; the git history is
    /// their record, not this doc.
    ///
    /// The number covers the MESSAGE shapes only — the request set, the reply set, the
    /// error set, and the shapes of the [`command`](crate::command) vocabulary. It does NOT
    /// cover which command names a host offers, any command's argument or reply schema, or
    /// the JSON text carried inside them: adding a command must never move it.
    /// `src/message/test/freeze.rs` is the pin that makes that a compile error rather than
    /// a convention.
    ///
    /// BOTH servers — the game's and the editor's — negotiate a `Hello` against this value.
    pub const CURRENT: Self = Self::new(14);

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
/// The [`HelloOk`](crate::message::QaResponse::HelloOk) payload. A struct (not a bare
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
