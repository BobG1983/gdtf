//! The editor's six prefab-canvas commands over its net-QA listener: `editor.map`,
//! `set_grid_size`, `select_tile`, `select_facing`, `set_level` and `paint`.
#![cfg(debug_assertions)]

mod canvas;
mod grid_command;
#[path = "../net_qa_shared/harness.rs"]
mod harness;
#[path = "../net_qa_shared/hello.rs"]
mod hello;
mod level_command;
#[path = "../net_qa_shared/load_case.rs"]
mod load_case;
mod map_command;
mod names;
#[path = "../net_qa_shared/outcome.rs"]
mod outcome;
mod paint_command;
mod pairing_command;
mod refusal;
mod rows;
mod select_facing_command;
mod select_tile_command;
mod setup;
#[path = "../net_qa_shared/socket.rs"]
mod socket;
#[path = "../net_qa_shared/support.rs"]
mod support;
mod tab_scope;
mod tiles;
#[path = "../net_qa_shared/world.rs"]
mod world;
