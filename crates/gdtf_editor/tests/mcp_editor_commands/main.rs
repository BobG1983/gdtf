//! The editor's theme helpers, its Injury sub-tab write, and the two host-level commands it
//! publishes under the game's own spellings — `capture.screenshot` and `wait` — driven over the
//! editor MCP listener.
#[cfg(not(feature = "mcp"))]
compile_error!("the mcp test suites need the host package's `mcp` feature");

#[path = "../mcp_shared/bad_arguments.rs"]
mod bad_arguments;
mod capture_screenshot_command;
mod delete_record_command;
mod drafts;
mod fixture_root;
#[path = "../mcp_shared/harness.rs"]
mod harness;
#[path = "../mcp_shared/hello.rs"]
mod hello;
mod keys;
#[path = "../mcp_shared/mirror.rs"]
mod mirror;
mod names;
#[path = "../mcp_shared/outcome.rs"]
mod outcome;
mod rows;
mod select_injury_tab_command;
mod select_theme_command;
mod set_default_floor_command;
mod setup;
#[path = "../mcp_shared/socket.rs"]
mod socket;
#[path = "../mcp_shared/support.rs"]
mod support;
mod toggle_terrain_command;
mod wait_command;
#[path = "../mcp_shared/world.rs"]
mod world;
