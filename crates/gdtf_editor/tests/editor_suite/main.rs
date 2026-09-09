//! The `gdtf_editor` integration suite.
mod authoring_validation;
mod content_shared;
mod delete;
mod editor_chrome;
mod load_and_roundtrip;
mod mcp_editor_authoring;
mod mcp_editor_commands;
mod mcp_editor_forms;
mod mcp_editor_prefab;
mod mcp_editor_reads;
mod mcp_editor_weighting;
mod mcp_hello;
mod mcp_shared;
mod melee_weapon_mode;
mod mode_shells;
mod prefab_mode;
mod state_scoped_resources;

#[cfg(not(feature = "mcp"))]
compile_error!("the mcp test suites need the host package's `mcp` feature");
