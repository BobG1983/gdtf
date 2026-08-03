//!   double-gates its wiring site on `cfg(all(debug_assertions, feature = "net_qa"))` and
mod channel;
mod config;
mod listener;
mod pending;

pub use channel::{IncomingRequest, NetInbox, Responder};
pub use config::{DEFAULT_IO_TIMEOUT, NetIoTimeout, NetQaPort};
pub use listener::{bind_listener, run_listener};
pub use pending::{PendingQueue, sweep_pending};
