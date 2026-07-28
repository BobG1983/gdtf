//! The MCP method semantics — `initialize`, the tool registry, and `tools/call`
//! (GTW-741, GTW-745).
//!
//! [`initialize`] builds the handshake result; [`tools`] enumerates the tools this bridge
//! exposes and their schemas; [`call`] maps a `tools/call` onto a request for the tool's
//! host and renders the reply; [`control`] handles the four host-local lifecycle tools
//! (`launch_game` / `stop_game` / `launch_editor` / `stop_editor`); [`launch_args`] reads
//! a launch call's recipe arguments into a [`LaunchSpec`](crate::lifecycle::LaunchSpec);
//! [`editor_topic`] maps the `query_editor` `topic` argument onto the protocol's topic
//! enum; [`content`] builds the shared reply content blocks. The JSON-RPC framing that
//! wraps these lives in [`rpc`](crate::rpc).

pub mod call;
pub mod content;
pub mod control;
pub mod editor_topic;
pub mod initialize;
pub mod launch_args;
pub mod tools;

pub use call::{ToolCallOutcome, handle_tool_call};
pub use control::{handle_launch, handle_stop};
pub use initialize::initialize_result;
pub use launch_args::parse_launch_spec;
pub use tools::{ToolName, tools_list_result};
