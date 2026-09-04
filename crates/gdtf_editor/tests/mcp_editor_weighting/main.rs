//! The Injury tab's weighting table over the wire: pick a table, read it, edit its rows, save it.
#[cfg(not(feature = "mcp"))]
compile_error!("the mcp test suites need the host package's `mcp` feature");

mod assets_root;
mod autoload;
#[path = "../mcp_shared/bad_arguments.rs"]
mod bad_arguments;
#[path = "../mcp_shared/harness.rs"]
mod harness;
#[path = "../mcp_shared/hello.rs"]
mod hello;
#[path = "../mcp_shared/mirror.rs"]
mod mirror;
mod names;
#[path = "../mcp_shared/outcome.rs"]
mod outcome;
mod refusal;
mod row_edits;
mod row_refusals;
mod rows;
mod save;
#[path = "../mcp_shared/save_fault.rs"]
mod save_fault;
mod select;
mod setup;
#[path = "../mcp_shared/socket.rs"]
mod socket;
#[path = "../mcp_shared/support.rs"]
mod support;
