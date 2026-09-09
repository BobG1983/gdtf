//! The editor's seven prefab-canvas commands over its MCP listener: `editor.map`,
//! `set_grid_size`, `select_tile`, `select_facing`, `set_level`, `paint` and `load_prefab`.

mod canvas;
mod grid_command;
mod level_command;
mod load_prefab_command;
mod map_command;
mod names;
mod paint_command;
mod pairing_command;
mod refusal;
mod rows;
mod select_facing_command;
mod select_tile_command;
mod setup;
mod tab_scope;
mod tiles;
