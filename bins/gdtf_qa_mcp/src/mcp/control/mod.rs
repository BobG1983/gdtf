//! The `launch_game` / `stop_game` tool handlers — starting and stopping the game process
//! (GTW-745; split into a directory module in GTW-875).
//!
//! These two tools are host-local: they do NOT forward a `QaRequest` to a running game
//! the way the other seven do — they start and stop the game process itself through the
//! [`GameLifecycle`](crate::lifecycle::GameLifecycle). On a successful launch the game
//! link is re-pointed at the port the child bound, so the following forwarding calls reach
//! it. Every path renders a normal MCP content block: a launch or stop failure is a tool
//! error, never a crash.
//!
//! - [`handle`] — the two entry points and the `port` argument.
//! - [`render`] — turning a launch / stop outcome into an MCP content block.

pub mod handle;
pub mod render;

#[cfg(test)]
mod test;

pub use handle::{handle_launch, handle_stop};
