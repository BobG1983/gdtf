//! The four launch / stop tool handlers — starting and stopping the game and editor
//! processes (GTW-745; split into a directory module in GTW-875; second host in GTW-808).
//!
//! These four tools are host-local: they do NOT forward a `QaRequest` to a running child
//! the way the twelve forwarding tools do — they start and stop the child process itself
//! through that host's [`HostLifecycle`](crate::lifecycle::HostLifecycle). On a successful
//! launch that host's link is re-pointed at the port the child bound, so the following
//! forwarding calls reach it. Every path renders a normal MCP content block: a launch or
//! stop failure is a tool error, never a crash.
//!
//! - [`handle`] — the four entry points and the `port` argument.
//! - [`render`] — turning a launch / stop outcome into an MCP content block.

pub mod handle;
pub mod render;

#[cfg(test)]
mod test;

pub use handle::{handle_launch, handle_stop};
