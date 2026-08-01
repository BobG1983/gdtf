//! The MCP method semantics — `initialize`, the tool registry, and `tools/call` (GTW-741,
//! GTW-745, GTW-943).
//!
//! [`initialize`] builds the handshake result; [`tools`] enumerates the five tools this
//! courier exposes and their schemas; [`control`] handles the three host-local ones
//! (`launch` / `stop` / `logs`); [`launch_args`] reads a launch call's recipe arguments into
//! a [`LaunchSpec`](crate::lifecycle::LaunchSpec); [`content`] builds the shared reply
//! content blocks. The JSON-RPC framing that wraps these lives in [`rpc`](crate::rpc).
//!
//! The crate-private `courier` module owns the `tools/call` dispatch and the two
//! command-layer tools (`commands` / `run`), which carry a host's own command vocabulary
//! without naming any of it, and `child_path` resolves a path a CHILD reported against the
//! directory that child ran in. Both are `pub(crate)`: nothing outside this binary calls
//! them, so they are named here in prose rather than linked (a public doc may not link a
//! private item).

pub(crate) mod child_path;
pub mod content;
pub mod control;
pub(crate) mod courier;
pub mod initialize;
pub mod launch_args;
pub mod tools;

pub use courier::{ToolCallOutcome, handle_tool_call};
pub use initialize::initialize_result;
pub use launch_args::parse_launch_spec;
pub use tools::{ToolName, tools_list_result};
