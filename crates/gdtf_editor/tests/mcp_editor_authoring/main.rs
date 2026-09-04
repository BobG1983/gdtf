//! One authoring session over the real editor socket: open a tab, blank its draft, write it
//! field by field and list by list, save it, and read the result back.
#[cfg(not(feature = "mcp"))]
compile_error!("the mcp test suites need the host package's `mcp` feature");

#[path = "../mcp_shared/draft_reply.rs"]
mod draft_reply;
#[path = "../mcp_shared/harness.rs"]
mod harness;
#[path = "../mcp_shared/hello.rs"]
mod hello;
#[path = "../mcp_shared/mirror.rs"]
mod mirror;
mod names;
#[path = "../mcp_shared/outcome.rs"]
mod outcome;
mod rows;
#[path = "../mcp_shared/save_fault.rs"]
mod save_fault;
mod session;
mod setup;
#[path = "../mcp_shared/socket.rs"]
mod socket;
#[path = "../mcp_shared/support.rs"]
mod support;
mod writes;
