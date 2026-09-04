//! Host-side MCP: the TCP socket, and command registration, admission and Bevy dispatch.

/// Build a command catalogue from erased commands and host facts.
pub mod catalogue;
/// Typed command trait and its erased form.
pub mod command;
/// Admit, claim, reply, and schedule command calls on a Bevy app.
pub mod dispatch;
/// Test-only fakes and assertions for host command sets.
pub mod test_support;
/// TCP transport for the MCP protocol between hosts and the MCP bridge.
pub mod transport;

pub use transport::{
    IncomingRequest, NetInbox, PendingQueue, Responder, bind_listener, run_listener, sweep_pending,
};
