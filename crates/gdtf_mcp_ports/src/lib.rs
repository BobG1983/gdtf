//! Loopback ports gdtf's two MCP hosts listen on. The game's port is fixed; the editor's
//! is a default the host may override.

mod ports;

pub use ports::{EDITOR_QA_PORT, EDITOR_QA_PORT_VAR, GAME_QA_PORT};
