//! Loopback ports for QA hosts. The game port is fixed; the editor port is a
//! default that `GDTF_EDITOR_NET_QA_PORT` overrides.

use bevy_derive::Deref;

/// Game net QA listen port (loopback). Not overridable by env.
pub const GAME_QA_PORT: u16 = 7616;

/// Content editor net QA listen port (loopback). Distinct from the game.
pub const EDITOR_QA_PORT: u16 = 7617;

/// Env var carrying the editor's net QA listen port, overriding [`EDITOR_QA_PORT`].
pub const EDITOR_QA_PORT_VAR: &str = "GDTF_EDITOR_NET_QA_PORT";

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
