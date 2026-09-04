//! The editor's seven prefab-canvas commands over its MCP listener: `editor.map`,
//! `set_grid_size`, `select_tile`, `select_facing`, `set_level`, `paint` and `load_prefab`.
#[cfg(not(feature = "mcp"))]
compile_error!("the mcp test suites need the host package's `mcp` feature");

mod canvas;
mod grid_command;
#[path = "../mcp_shared/harness.rs"]
mod harness;
#[path = "../mcp_shared/hello.rs"]
mod hello;
mod level_command;
#[path = "../mcp_shared/load_case.rs"]
mod load_case;
mod load_prefab_command;
mod map_command;
mod names;
#[path = "../mcp_shared/outcome.rs"]
mod outcome;
mod paint_command;
mod pairing_command;
mod refusal;
mod rows;
mod select_facing_command;
mod select_tile_command;
mod setup;
#[path = "../mcp_shared/socket.rs"]
mod socket;
#[path = "../mcp_shared/support.rs"]
mod support;
mod tab_scope;
mod tiles;
#[path = "../mcp_shared/world.rs"]
mod world;
