//! Shared loopback ports for QA hosts. Not overridable by env.

use bevy_derive::Deref;

/// Game net QA listen port (loopback).
pub const GAME_QA_PORT: u16 = 7616;

/// Content editor net QA listen port (loopback). Distinct from the game.
pub const EDITOR_QA_PORT: u16 = 7617;

/// TCP port used by a net QA listener or client.
#[derive(Deref, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct NetQaPort(u16);

impl NetQaPort {
    /// Wrap a port number.
    #[must_use]
    pub const fn new(port: u16) -> Self {
        Self(port)
    }
}
