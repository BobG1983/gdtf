//! The two loopback ports, and the env var the editor's may be overridden with.

/// Game MCP listen port (loopback). Not overridable by env.
pub const GAME_QA_PORT: u16 = 7616;

/// Content editor MCP listen port (loopback). Distinct from the game.
pub const EDITOR_QA_PORT: u16 = 7617;

/// Env var carrying the editor's MCP listen port, overriding [`EDITOR_QA_PORT`].
pub const EDITOR_QA_PORT_VAR: &str = "EDITOR_MCP_PORT";
