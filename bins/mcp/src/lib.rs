//! This game's QA MCP bridge: the hosts it drives and the identity it advertises.

/// The game and editor hosts this bridge registers.
pub mod hosts;
/// What this bridge advertises itself as.
pub mod identity;
/// Entry point that serves the registered hosts.
pub mod serve;

pub use hosts::registry;
pub use identity::identity;
pub use serve::run;
