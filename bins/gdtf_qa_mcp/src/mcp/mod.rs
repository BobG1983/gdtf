//! The MCP method semantics — `initialize`, the tool registry, and `tools/call`
//! (GTW-741).
//!
//! [`initialize`] builds the handshake result; [`tools`] enumerates the five tools this
//! bridge exposes and their schemas; [`call`] maps a `tools/call` onto a game request and
//! renders the reply. The JSON-RPC framing that wraps these lives in
//! [`rpc`](crate::rpc).

pub mod call;
pub mod initialize;
pub mod tools;

pub use call::{ToolCallOutcome, handle_tool_call};
pub use initialize::initialize_result;
pub use tools::{ToolName, tools_list_result};
