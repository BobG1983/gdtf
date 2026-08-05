//! TCP transport for the net QA protocol between game/editor hosts and the MCP bridge.

mod channel;
mod config;
mod listener;
mod pending;

pub use channel::{IncomingRequest, NetInbox, Responder};
pub use config::{
    DEFAULT_IO_TIMEOUT, DEFAULT_REPLY_TIMEOUT, NetIoTimeout, NetQaPort, NetReplyTimeout,
    NetTimeouts,
};
pub use listener::{bind_listener, run_listener};
pub use pending::{PendingQueue, sweep_pending};
