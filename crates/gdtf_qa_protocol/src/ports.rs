//! Shared loopback ports for QA hosts. Not overridable by env.

/// Game net QA listen port (loopback).
pub const GAME_QA_PORT: u16 = 7616;

/// Content editor net QA listen port (loopback). Distinct from the game.
pub const EDITOR_QA_PORT: u16 = 7617;
