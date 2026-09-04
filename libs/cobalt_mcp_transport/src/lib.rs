//! TCP transport for the MCP protocol between game/editor hosts and the MCP bridge.

mod channel;
mod listener;
mod pending;

pub use channel::{IncomingRequest, NetInbox, Responder};
pub use listener::{bind_listener, run_listener};
pub use pending::{PendingQueue, sweep_pending};
