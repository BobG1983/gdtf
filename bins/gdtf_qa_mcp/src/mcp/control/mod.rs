//! The three host-local tool handlers — starting and stopping the two child processes, and
//! reading what a running one printed (GTW-745; split into a directory module in GTW-875;
//! second host in GTW-808; `logs` in GTW-943).
//!
//! These three are host-local: they do NOT carry a request to a running child the way
//! `commands` and `run` do — they drive the child process itself through that host's
//! [`HostLifecycle`](crate::lifecycle::HostLifecycle). On a successful launch that host's
//! link is re-pointed at the port the child bound, so the following calls reach it. Every
//! path renders a normal MCP content block: a failure is a tool error, never a crash.
//!
//! - [`handle`] — the three entry points and the `port` argument.
//! - [`render`] — turning a launch / stop / logs outcome into an MCP content block.

pub mod handle;
pub mod render;

#[cfg(test)]
mod test;

pub use handle::{handle_launch, handle_logs, handle_stop};
