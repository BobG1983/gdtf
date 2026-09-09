//! The game crate's integration suite: one binary, one module per suite.
mod action_bar;
mod battle_bootstrap;
mod battle_running_driver;
mod battle_shell;
mod battle_sim;
mod capstone;
mod combat_log;
mod contextual_panel;
mod load_families;
mod mcp;
mod migrated_content;
mod playback_gate;
#[cfg(feature = "dev_tools")]
mod procgen_stepper;
mod shell_scenes;
mod state_walk;
mod status_panel;
mod weapon_panel;

#[cfg(not(feature = "mcp"))]
compile_error!("the mcp test suites need the host package's `mcp` feature");
