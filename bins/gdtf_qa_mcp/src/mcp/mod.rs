//! The MCP method semantics — `initialize`, the tool registry, and `tools/call`
//! (GTW-741, GTW-745).
//!
//! [`initialize`] builds the handshake result; [`tools`] enumerates the tools this bridge
//! exposes and their schemas; [`call`] maps a `tools/call` onto a game request and renders
//! the reply; [`control`] handles the two host-local lifecycle tools (`launch_game` /
//! `stop_game`); [`content`] builds the shared reply content blocks. The JSON-RPC framing
//! that wraps these lives in [`rpc`](crate::rpc).

pub mod call;
pub mod content;
pub mod control;
pub mod initialize;
pub mod tools;

pub use call::{ToolCallOutcome, handle_tool_call};
pub use control::{handle_launch, handle_stop};
pub use initialize::initialize_result;
pub use tools::{ToolName, tools_list_result};
