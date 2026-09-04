//! MCP protocol handlers: initialize, tools list, tool calls.

pub(crate) mod child_path;
/// MCP content helpers for tool replies.
pub mod content;
/// Launch / stop / logs tool handlers.
pub mod control;
pub(crate) mod courier;
/// `initialize` result.
pub mod initialize;
/// Parse launch tool arguments into a [`crate::lifecycle::launch::LaunchSpec`].
pub mod launch_args;
/// Tool names and list schema.
pub mod tools;

pub use courier::{InstanceChoice, ToolCallOutcome, handle_tool_call, resolve_instance};
pub use initialize::{ServerIdentity, ServerName, ServerVersion, initialize_result};
pub use launch_args::parse_launch_spec;
pub use tools::{ToolName, tools_list_result};
