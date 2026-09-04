//! The editor's four shared reads: validation, families, session and draft, over the editor
//! MCP listener.
#[cfg(not(feature = "mcp"))]
compile_error!("the mcp test suites need the host package's `mcp` feature");

#[path = "../mcp_shared/bad_arguments.rs"]
mod bad_arguments;
mod draft_command;
#[path = "../mcp_shared/draft_reply.rs"]
mod draft_reply;
mod families_command;
#[path = "../mcp_shared/harness.rs"]
mod harness;
#[path = "../mcp_shared/hello.rs"]
mod hello;
#[path = "../mcp_shared/load_case.rs"]
mod load_case;
#[path = "../mcp_shared/mirror.rs"]
mod mirror;
mod names;
#[path = "../mcp_shared/outcome.rs"]
mod outcome;
mod rows;
#[path = "../mcp_shared/save_fault.rs"]
mod save_fault;
mod session_command;
#[path = "../mcp_shared/socket.rs"]
mod socket;
#[path = "../mcp_shared/support.rs"]
mod support;
mod validation_command;
