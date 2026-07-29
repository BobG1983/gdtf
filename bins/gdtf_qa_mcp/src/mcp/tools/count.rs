//! The tool-set counts, computed from `ALL` when the crate compiles (GTW-905).
//!
//! The module doc links these constants instead of writing the numbers itself, so adding
//! or removing a [`ToolName`](super::ToolName) variant moves the documented count with it
//! and no number is ever copied out of a ticket.

use std::ops::Deref;

use crate::mcp::tools::name::ALL;

/// A number of MCP tools.
///
/// Private-inner newtype over `usize` (no-bare-types): the size of the tool set is a
/// domain quantity, not a bare length.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ToolCount(usize);

impl ToolCount {
    /// Build a count from a number of tools.
    #[must_use]
    pub const fn new(count: usize) -> Self {
        Self(count)
    }
}

impl Deref for ToolCount {
    type Target = usize;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

/// Every tool the bridge exposes — the length of `ALL`.
pub const TOOL_COUNT: ToolCount = ToolCount::new(ALL.len());

/// The tools that start or stop a child process instead of reaching a wire.
pub const HOST_LOCAL_TOOL_COUNT: ToolCount = ToolCount::new(host_local());

/// The tools that forward a request to a running child.
pub const FORWARDING_TOOL_COUNT: ToolCount = ToolCount::new(ALL.len() - host_local());

/// Walk `ALL` and count the launch / stop tools.
///
/// A `while` loop over the slice rather than an iterator chain, because a `const`
/// initializer cannot call `Iterator::filter` / `count`.
const fn host_local() -> usize {
    let mut count = 0;
    let mut index = 0;
    while index < ALL.len() {
        if ALL[index].is_launch() || ALL[index].is_stop() {
            count += 1;
        }
        index += 1;
    }
    count
}
